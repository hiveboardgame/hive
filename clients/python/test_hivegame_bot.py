"""Checks the frames this client builds against the shapes in BOT_WEBSOCKET_API.md.

Run: python3 test_hivegame_bot.py
No server needed; the socket is a stub that records what would have been sent.
"""

import asyncio
import json

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


def connection_through(socket):
    """The bot and the socket its `connect` will reach. The socket is handed back
    separately because a connect that cleans up after itself drops the bot's reference to
    it, and the whole question is whether that socket was hung up."""

    async def stub_connect(_url):
        return socket

    hivegame_bot.websockets = type("stub", (), {"connect": staticmethod(stub_connect)})
    return HiveBot("tok"), socket


def bot_connecting_through(incoming):
    """A bot whose `connect` reaches a stub instead of a server."""
    bot, _ = connection_through(FakeSocket(incoming))
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
    bot = bot_with(incoming)
    messages = bot.messages()
    first = await messages.__anext__()

    assert first == {"Game": {"Reaction": {"kind": "move"}}}, first
    assert frames(bot) == [{"Pong": 7}], frames(bot)


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
    class Cancelling(FakeSocket):
        async def recv(self):
            raise asyncio.CancelledError

    bot, socket = connection_through(Cancelling())

    try:
        await bot.connect()
    except asyncio.CancelledError:
        pass
    else:
        raise AssertionError("cancellation must propagate")

    assert socket.closed, "a cancelled connect left the socket open"


async def check_a_successful_connect_keeps_the_socket():
    # The guard against fixing the leak with an unconditional hang-up.
    bot, socket = connection_through(FakeSocket([AUTHENTICATED]))

    await bot.connect()

    assert not socket.closed, "a successful connect must leave the socket usable"


async def main():
    await check_requests()
    await check_auth_frame_and_envelope_unwrapping()
    await check_pings_are_answered_and_not_yielded()
    await check_auth_survives_traffic_arriving_before_the_ack()
    await check_an_anonymous_snapshot_is_not_authentication()
    await check_a_refused_token_raises()
    await check_a_refused_token_closes_the_socket()
    await check_a_failed_context_manager_closes_the_socket()
    await check_a_cancelled_connect_closes_the_socket()
    await check_a_successful_connect_keeps_the_socket()
    print("all frame checks passed")


if __name__ == "__main__":
    asyncio.run(main())
