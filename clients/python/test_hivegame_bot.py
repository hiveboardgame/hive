"""Checks the frames this client builds against the shapes in BOT_WEBSOCKET_API.md.

Run: python3 test_hivegame_bot.py
No server needed; the socket is a stub that records what would have been sent.
"""

import asyncio
import io
import json
import urllib.error

import hivegame_bot
from hivegame_bot import HiveBot, HiveError


class FakeSocket:
    def __init__(self, incoming=()):
        self.sent = []
        self.closed = False
        self._incoming = list(incoming)

    async def send(self, text):
        self.sent.append(text)

    async def recv(self):
        if not self._incoming:
            # A real socket with nothing to say blocks rather than failing.
            await asyncio.Event().wait()
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
    await bot.play("abc", "wA1 -bQ", turn=12)
    await bot.control("abc", "Resign", "White")
    await bot.join("abc")
    await bot.get_game("abc")
    await bot.get_pending_games()
    await bot.get_ongoing_games()
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
        {"Game": {"game_id": "abc", "action": {"PlayAt": {"turn": 12, "notation": "wA1 -bQ"}}}},
        {"Game": {"game_id": "abc", "action": {"Control": {"Resign": "White"}}}},
        {"Game": {"game_id": "abc", "action": "Join"}},
        {"GetGame": "abc"},
        "GetPendingGames",
        "GetOngoingGames",
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
    assert await bot._receive(bot._socket) == {"LobbySnapshot": {"urgent_games": []}}


async def check_a_refusal_is_yielded_not_raised():
    refused = {"Game": {"game_id": "abc", "reason": "Invalid action: It is not this player's turn"}}
    bot = await connected([json.dumps({"Err": refused}), ok({"Game": {"Reaction": {}}})])

    stream = bot.messages()
    assert await stream.__anext__() == {"Err": refused}
    assert await stream.__anext__() == {"Game": {"Reaction": {}}}


def connection_through(socket):
    """The bot and the socket its `connect` will reach. The socket is handed back
    separately because a connect that cleans up after itself drops the bot's reference to
    it, and the whole question is whether that socket was hung up."""

    async def stub_connect(_url, **options):
        socket.options = options
        return socket

    hivegame_bot.websockets = type("stub", (), {"connect": staticmethod(stub_connect)})
    return HiveBot("tok"), socket


def bot_connecting_through(incoming):
    """A bot whose `connect` reaches a stub instead of a server."""
    bot, _ = connection_through(FakeSocket(incoming))
    return bot


async def connected(incoming=()):
    bot = bot_connecting_through([AUTHENTICATED, *incoming])
    await bot.connect()
    return bot


def ok(message):
    return json.dumps({"Ok": message})


AUTHENTICATED = ok({"Authenticated": {"username": "bot", "uid": "u-1"}})
ONLINE = ok({"UserStatus": {"status": "Online", "username": "bot", "user": None}})
SNAPSHOT = ok({"LobbySnapshot": {"urgent_games": []}})


async def check_auth_survives_traffic_arriving_before_the_ack():
    # The socket is subscribed to the lobby before it authenticates, and the server
    # broadcasts the bot's own Online ahead of the ack, so neither is a failure.
    bot = bot_connecting_through([ONLINE, ok({"Ping": {"nonce": 7, "value": 1.0}}), AUTHENTICATED])

    assert await bot.connect() == {"username": "bot", "uid": "u-1"}
    assert frames(bot) == [{"Auth": "tok"}, {"Pong": 7}], frames(bot)

    first = await bot.messages().__anext__()
    assert first == {"UserStatus": {"status": "Online", "username": "bot", "user": None}}, first


async def check_an_anonymous_snapshot_is_not_authentication():
    # A connection starts anonymous and gets its own snapshot; only the ack says the
    # Auth frame landed.
    bot = bot_connecting_through([SNAPSHOT, AUTHENTICATED])

    assert await bot.connect() == {"username": "bot", "uid": "u-1"}

    first = await bot.messages().__anext__()
    assert first == {"LobbySnapshot": {"urgent_games": []}}, first


async def check_a_refused_token_raises():
    bot = bot_connecting_through([ok({"Error": "Auth failed"})])

    try:
        await bot.connect()
    except HiveError:
        return
    raise AssertionError("a refused token must raise")


async def check_pings_are_answered_and_not_yielded():
    incoming = [
        json.dumps({"Ok": {"Ping": {"nonce": 7, "value": 1.0}}}),
        json.dumps({"Ok": {"Game": {"Reaction": {"kind": "move"}}}}),
    ]
    bot = await connected(incoming)
    messages = bot.messages()
    first = await messages.__anext__()

    assert first == {"Game": {"Reaction": {"kind": "move"}}}, first
    assert frames(bot) == [{"Auth": "tok"}, {"Pong": 7}], frames(bot)


async def check_pings_are_answered_while_the_bot_is_busy():
    # A bot awaiting its engine reads nothing. The server hangs up on a socket that stops
    # answering, and books a latency ping answered late as lag.
    bot = await connected(
        [
            ok({"Game": {"Reaction": {"kind": "move"}}}),
            ok({"Ping": {"nonce": 1, "value": 0}}),
            ok({"Ping": {"nonce": 2, "value": 0}}),
        ]
    )

    await bot.messages().__anext__()
    await asyncio.sleep(0.01)

    assert frames(bot) == [{"Auth": "tok"}, {"Pong": 1}, {"Pong": 2}], frames(bot)


async def check_a_dropped_connection_ends_the_message_loop():
    # The reader runs on its own, so a failure there has to reach the bot rather than leave
    # it waiting on a socket that is gone.
    class Dropping(FakeSocket):
        async def recv(self):
            if self._incoming:
                return self._incoming.pop(0)
            raise ConnectionError("gone")

    bot, _ = connection_through(Dropping([AUTHENTICATED, ok({"Game": {"Reaction": {}}})]))
    await bot.connect()
    messages = bot.messages()

    assert await messages.__anext__() == {"Game": {"Reaction": {}}}
    for _ in range(2):
        try:
            await bot._next()
        except ConnectionError:
            continue
        raise AssertionError("a dropped connection must raise, every time")


async def check_a_refused_token_closes_the_socket():
    # `__aenter__` is `connect`, so a raise there means `__aexit__` never runs and nothing
    # else will ever hang up. The server keeps the connection open after "Auth failed"
    # rather than closing it, so a bot retrying a bad token leaks one socket per attempt.
    bot, socket = connection_through(FakeSocket([ok({"Error": "Auth failed"})]))

    try:
        await bot.connect()
    except HiveError:
        pass
    else:
        raise AssertionError("a refused token must raise")

    assert socket.closed, "a refused token left the socket open"
    assert bot._socket is None, "a bot that failed to connect must not look connected"
    assert bot._reader is None, "a bot that failed to connect must stop reading"


async def check_a_failed_context_manager_closes_the_socket():
    bot, socket = connection_through(FakeSocket([ok({"Error": "Auth failed"})]))

    try:
        async with bot:
            raise AssertionError("the body must not run when authentication failed")
    except HiveError:
        pass

    assert socket.closed, "async with leaked the socket when authentication failed"


async def check_a_cancelled_connect_closes_the_socket():
    # Same shape as a refusal: a bot that times out its own connect, or is cancelled at
    # shutdown, unwinds out of `connect` with the socket already open.
    bot, socket = connection_through(FakeSocket())

    try:
        await asyncio.wait_for(bot.connect(), timeout=0.01)
    except asyncio.TimeoutError:
        pass
    else:
        raise AssertionError("a server that never answers must time the connect out")

    assert socket.closed, "a cancelled connect left the socket open"
    assert bot._reader is None, "a cancelled connect left the reader running"


async def check_large_frames_are_accepted():
    # 1,951 ongoing games measured 8 MB in a single frame.
    bot, socket = connection_through(FakeSocket([AUTHENTICATED]))

    await bot.connect()

    assert socket.options.get("max_size", 1 << 20) is None, socket.options


async def check_a_successful_connect_keeps_the_socket():
    # The guard against fixing the leak with an unconditional hang-up.
    bot, socket = connection_through(FakeSocket([AUTHENTICATED]))

    await bot.connect()

    assert not socket.closed, "a successful connect must leave the socket usable"


def check_a_refused_login_keeps_the_servers_reason():
    body = json.dumps({"success": False, "data": {"message": "Wrong password"}}).encode()

    def refuse(_request):
        raise urllib.error.HTTPError("url", 400, "Bad Request", {}, io.BytesIO(body))

    real_urlopen = hivegame_bot.urllib.request.urlopen
    hivegame_bot.urllib.request.urlopen = refuse
    try:
        hivegame_bot.fetch_token("bot@example.com", "nope")
    except HiveError as error:
        assert "Wrong password" in str(error), error
    else:
        raise AssertionError("a refused login must raise")
    finally:
        hivegame_bot.urllib.request.urlopen = real_urlopen


async def main():
    check_a_refused_login_keeps_the_servers_reason()
    await check_requests()
    await check_auth_frame_and_envelope_unwrapping()
    await check_pings_are_answered_and_not_yielded()
    await check_pings_are_answered_while_the_bot_is_busy()
    await check_a_dropped_connection_ends_the_message_loop()
    await check_a_refusal_is_yielded_not_raised()
    await check_auth_survives_traffic_arriving_before_the_ack()
    await check_an_anonymous_snapshot_is_not_authentication()
    await check_a_refused_token_raises()
    await check_a_refused_token_closes_the_socket()
    await check_a_failed_context_manager_closes_the_socket()
    await check_a_cancelled_connect_closes_the_socket()
    await check_a_successful_connect_keeps_the_socket()
    await check_large_frames_are_accepted()
    print("all frame checks passed")


if __name__ == "__main__":
    asyncio.run(main())
