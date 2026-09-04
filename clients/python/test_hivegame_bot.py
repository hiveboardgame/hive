"""Checks the frames this client builds against the shapes in BOT_WEBSOCKET_API.md.

Run: python3 test_hivegame_bot.py
No server needed; the socket is a stub that records what would have been sent.
"""

import asyncio
import json

from hivegame_bot import HiveBot


class FakeSocket:
    def __init__(self, incoming=()):
        self.sent = []
        self._incoming = list(incoming)

    async def send(self, text):
        self.sent.append(text)

    async def recv(self):
        return self._incoming.pop(0)

    async def close(self):
        pass


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


async def main():
    await check_requests()
    await check_auth_frame_and_envelope_unwrapping()
    await check_pings_are_answered_and_not_yielded()
    print("all frame checks passed")


if __name__ == "__main__":
    asyncio.run(main())
