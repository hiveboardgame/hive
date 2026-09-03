"""Checks the frames this client builds against the shapes in BOT_WEBSOCKET_API.md.

Run: python3 test_hivegame_bot.py
No server needed; the socket is a stub that records what would have been sent.
"""

import asyncio
import io
import json
import urllib.error
import urllib.request

import hivegame_bot
from hivegame_bot import HiveBot, HiveError, fetch_token


class FakeSocket:
    def __init__(self, incoming=()):
        self.sent = []
        self.closed = False
        self._incoming = list(incoming)

    async def send(self, text):
        self.sent.append(text)

    async def recv(self):
        return self._incoming.pop(0)

    async def close(self):
        self.closed = True


def bot_with(incoming=()):
    bot = HiveBot("tok")
    bot._socket = FakeSocket(incoming)
    return bot


def frames(bot):
    return [json.loads(text) for text in bot._socket.sent]


async def check_requests():
    bot = bot_with()
    await bot.play("abc", "wA1 -bQ")
    await bot.control("abc", "Resign", "White")
    await bot.join("abc")
    await bot.get_game("abc")
    await bot.get_pending_games()
    await bot.get_username("someone")
    await bot.accept_challenge("xyz")
    await bot.create_challenge(
        rated=True,
        game_type="MLP",
        visibility="Public",
        opponent=None,
        color_choice="Random",
        time_mode="RealTime",
        time_base=300,
        time_increment=3,
        band_upper=None,
        band_lower=None,
    )

    assert frames(bot) == [
        {"Game": {"game_id": "abc", "action": {"Play": "wA1 -bQ"}}},
        {"Game": {"game_id": "abc", "action": {"Control": {"Resign": "White"}}}},
        {"Game": {"game_id": "abc", "action": "Join"}},
        {"GetGame": "abc"},
        "GetPendingGames",
        {"GetUsername": "someone"},
        {"Challenge": {"Accept": "xyz"}},
        {
            "Challenge": {
                "Create": {
                    "rated": True,
                    "game_type": "MLP",
                    "visibility": "Public",
                    "opponent": None,
                    "color_choice": "Random",
                    "time_mode": "RealTime",
                    "time_base": 300,
                    "time_increment": 3,
                    "band_upper": None,
                    "band_lower": None,
                }
            }
        },
    ], frames(bot)


async def check_auth_frame_and_envelope_unwrapping():
    bot = bot_with([json.dumps({"Ok": {"LobbySnapshot": {"urgent_games": []}}})])

    await bot.send({"Auth": bot._token})
    assert frames(bot) == [{"Auth": "tok"}], frames(bot)
    assert await bot._receive() == {"LobbySnapshot": {"urgent_games": []}}


async def check_pings_are_answered_and_not_yielded():
    incoming = [
        json.dumps({"Ok": {"Ping": {"nonce": 7, "value": 1.0}}}),
        json.dumps({"Ok": {"Game": {"Reaction": {"kind": "move"}}}}),
    ]
    bot = bot_with(incoming)
    messages = bot.messages()
    first = await messages.__anext__()

    assert first == {"Game": {"Reaction": {"kind": "move"}}}, first
    assert frames(bot) == [{"Pong": 7}], frames(bot)


def ok(message):
    return json.dumps({"Ok": message})


async def connect_against(incoming):
    """Runs connect() over a stub socket, skipping the real websockets handshake."""
    bot = HiveBot("tok")
    socket = FakeSocket(incoming)

    async def fake_connect(url):
        return socket

    original = hivegame_bot.websockets.connect
    hivegame_bot.websockets.connect = fake_connect
    try:
        return bot, socket, await bot.connect()
    finally:
        hivegame_bot.websockets.connect = original


async def check_auth_survives_an_interleaved_ping():
    # jobs/ping.rs pings every socket once a second, so a Ping lands on the bot while the
    # server is still doing the two database round-trips that Auth needs.
    bot, socket, _ = await connect_against(
        [
            ok({"Ping": {"nonce": 7, "value": 1.0}}),
            ok({"UserStatus": {"status": "Online", "username": "bot", "user": None}}),
            ok({"AuthOk": {"username": "bot"}}),
            ok({"LobbySnapshot": {"urgent_games": []}}),
        ]
    )
    assert {"Pong": 7} in frames(bot), frames(bot)


async def check_an_anonymous_snapshot_is_not_proof_of_auth():
    # The connect-time snapshot is sent before Auth is even read, so a snapshot alone says
    # nothing about whether the token was accepted.
    try:
        await connect_against(
            [
                ok({"LobbySnapshot": {"urgent_games": []}}),
                ok({"Error": "Auth failed"}),
            ]
        )
    except HiveError:
        return
    raise AssertionError("an unauthenticated snapshot was accepted as successful auth")


async def check_a_failed_auth_closes_the_socket():
    bot = HiveBot("tok")
    socket = FakeSocket([ok({"Error": "Auth failed"})])

    async def fake_connect(url):
        return socket

    original = hivegame_bot.websockets.connect
    hivegame_bot.websockets.connect = fake_connect
    try:
        async with bot:
            pass
    except HiveError:
        pass
    finally:
        hivegame_bot.websockets.connect = original

    assert socket.closed, "a rejected token must not leak the open socket"


def check_a_rejected_token_raises_hive_error():
    # The token endpoint answers 400, which urlopen turns into HTTPError before any of the
    # client's own error handling runs.
    def fake_urlopen(request):
        raise urllib.error.HTTPError(
            request.full_url,
            400,
            "Bad Request",
            {},
            io.BytesIO(json.dumps({"success": False, "data": {"message": "Not a bot"}}).encode()),
        )

    original = urllib.request.urlopen
    urllib.request.urlopen = fake_urlopen
    try:
        fetch_token("bot@example.com", "wrong")
    except HiveError as error:
        assert "Not a bot" in str(error), error
        return
    finally:
        urllib.request.urlopen = original
    raise AssertionError("a rejected token did not surface as HiveError")


CHECKS = (
    check_requests,
    check_auth_frame_and_envelope_unwrapping,
    check_pings_are_answered_and_not_yielded,
    check_auth_survives_an_interleaved_ping,
    check_an_anonymous_snapshot_is_not_proof_of_auth,
    check_a_failed_auth_closes_the_socket,
    check_a_rejected_token_raises_hive_error,
)


async def main():
    failed = 0
    for check in CHECKS:
        try:
            result = check()
            if asyncio.iscoroutine(result):
                await result
        except Exception as error:
            failed += 1
            print(f"FAIL {check.__name__}: {type(error).__name__}: {error}")
        else:
            print(f"ok   {check.__name__}")
    if failed:
        raise SystemExit(f"{failed} check(s) failed")
    print("all frame checks passed")


if __name__ == "__main__":
    asyncio.run(main())
