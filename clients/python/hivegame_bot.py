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
import urllib.error
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
    try:
        with urllib.request.urlopen(request) as response:
            body = json.load(response)
    except urllib.error.HTTPError as error:
        # Rejected credentials answer 400 with the reason in the body, and urlopen raises
        # on 4xx before that body can be read as a normal response.
        try:
            body = json.load(error)
        except ValueError:
            raise HiveError(f"token request failed: HTTP {error.code}") from error
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
        try:
            await self.connect()
        except BaseException:
            # connect() opened the socket before it raised, and a failing __aenter__ never
            # reaches __aexit__.
            await self.close()
            raise
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()

    async def connect(self) -> dict[str, Any]:
        """Authenticates, then returns the lobby snapshot: the games and challenges you
        already have.

        The server starts sending the moment it accepts the socket and pings every second,
        so anything before `AuthOk` belongs to the connection's pre-authentication life. Only
        `AuthOk` proves the token was accepted; a snapshot on its own does not, because one is
        sent to unauthenticated sockets too.
        """
        self._socket = await websockets.connect(self._url)
        await self.send({"Auth": self._token})
        await self._await_auth()
        return await self._await_snapshot()

    async def _await_auth(self) -> None:
        while True:
            message = await self._receive()
            if "AuthOk" in message:
                return
            error = message.get("Error")
            if error is not None:
                raise HiveError(f"authentication failed: {error}")
            await self._answer_ping(message)

    async def _await_snapshot(self) -> dict[str, Any]:
        while True:
            message = await self._receive()
            snapshot = message.get("LobbySnapshot")
            if snapshot is not None:
                return snapshot
            await self._answer_ping(message)

    async def _answer_ping(self, message: dict[str, Any]) -> bool:
        ping = message.get("Ping")
        if ping is None:
            return False
        await self.send({"Pong": ping["nonce"]})
        return True

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

    async def get_ongoing_games(self) -> None:
        """Every unfinished game you play, whoever is to move. The lobby snapshot carries only
        the ones waiting on you."""
        await self.send("GetOngoingGames")

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
            if await self._answer_ping(message):
                continue
            yield message
