"""A small client for the hivegame.com bot WebSocket API.

One file, one dependency (`websockets`). Copy it into your project.

The wire format is documented in BOT_WEBSOCKET_API.md; everything here is a thin wrapper
over it, so if a frame you need is missing, `send` takes the raw thing.

    import asyncio
    from hivegame_bot import HiveBot, fetch_token

    async def main():
        token = fetch_token("bot@example.com", "hunter2")
        async with HiveBot(token) as bot:
            async for message in bot.messages():
                if "Game" in message:
                    ...  # your opponent moved

    asyncio.run(main())
"""

from __future__ import annotations

import json
import urllib.request
from typing import Any, AsyncIterator, Iterable

import websockets

DEFAULT_HOST = "hivegame.com"


class HiveError(Exception):
    """The server refused something, or the token was rejected."""


def fetch_token(
    email: str,
    password: str,
    host: str = DEFAULT_HOST,
    secure: bool = True,
) -> str:
    """Credentials in, JWT out. The only HTTP request a bot makes."""
    scheme = "https" if secure else "http"
    request = urllib.request.Request(
        f"{scheme}://{host}/api/v1/auth/token",
        data=json.dumps({"email": email, "password": password}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request) as response:
        body = json.load(response)
    if not body.get("success"):
        raise HiveError(body.get("data", {}).get("message", "token request failed"))
    return body["data"]["token"]


class HiveBot:
    def __init__(self, token: str, host: str = DEFAULT_HOST, secure: bool = True) -> None:
        self._token = token
        scheme = "wss" if secure else "ws"
        self._url = f"{scheme}://{host}/ws/?format=json"
        self._socket: Any = None

    async def __aenter__(self) -> HiveBot:
        await self.connect()
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()

    async def connect(self) -> dict[str, Any]:
        """Returns the lobby snapshot: the games and challenges you already have."""
        self._socket = await websockets.connect(self._url)
        await self.send({"Auth": self._token})
        message = await self._receive()
        if "LobbySnapshot" not in message:
            raise HiveError(f"authentication failed: {message}")
        return message["LobbySnapshot"]

    async def close(self) -> None:
        if self._socket is not None:
            await self._socket.close()
            self._socket = None

    async def send(self, frame: Any) -> None:
        """Every method below is one call to this. Use it for frames they do not cover."""
        if self._socket is None:
            raise HiveError("not connected")
        await self._socket.send(json.dumps(frame))

    async def _game(self, game_id: str, action: Any) -> None:
        await self.send({"Game": {"game_id": game_id, "action": action}})

    async def play(self, game_id: str, notation: str) -> None:
        """UHP notation, e.g. `wA1 -bQ`. The first move of a game is just the piece."""
        await self._game(game_id, {"Play": notation})

    async def control(self, game_id: str, control: str, color: str) -> None:
        """`control` is Resign, DrawOffer, DrawAccept, DrawReject, TakebackRequest,
        TakebackAccept, TakebackReject or Abort. `color` is your side."""
        await self._game(game_id, {"Control": {control: color}})

    async def join(self, game_id: str) -> None:
        await self._game(game_id, "Join")

    async def unwatch(self, game_id: str) -> None:
        await self._game(game_id, "Unwatch")

    async def create_challenge(self, **details: Any) -> None:
        await self.send({"Challenge": {"Create": details}})

    async def accept_challenge(self, challenge_id: str) -> None:
        await self.send({"Challenge": {"Accept": challenge_id}})

    async def delete_challenge(self, challenge_id: str) -> None:
        await self.send({"Challenge": {"Delete": challenge_id}})

    async def delete_challenges(self, challenge_ids: Iterable[str]) -> None:
        await self.send({"Challenge": {"DeleteMany": list(challenge_ids)}})

    async def get_game(self, game_id: str) -> None:
        """Any game, including other people's. Does not subscribe you to it."""
        await self.send({"GetGame": game_id})

    async def get_pending_games(self) -> None:
        await self.send("GetPendingGames")

    async def get_user(self, user_id: str) -> None:
        await self.send({"GetUser": user_id})

    async def get_username(self, username: str) -> None:
        await self.send({"GetUsername": username})

    async def resync(self) -> None:
        await self.send("Resync")

    async def _receive(self) -> dict[str, Any]:
        if self._socket is None:
            raise HiveError("not connected")
        envelope = json.loads(await self._socket.recv())
        if "Err" in envelope:
            raise HiveError(envelope["Err"])
        return envelope["Ok"]

    async def messages(self) -> AsyncIterator[dict[str, Any]]:
        """Yield server messages forever, answering latency pings on the way through."""
        while True:
            message = await self._receive()
            ping = message.get("Ping")
            if ping is not None:
                await self.send({"Pong": ping["nonce"]})
                continue
            yield message
