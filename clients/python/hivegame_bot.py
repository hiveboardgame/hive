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
                if "Err" in message:
                    ...  # a request was refused; message["Err"] says which and why
                elif "Game" in message:
                    ...  # your opponent moved

    asyncio.run(main())
"""

from __future__ import annotations

import asyncio
import contextlib
import json
import urllib.error
import urllib.request
from typing import Any, AsyncIterator, Iterable

import websockets

DEFAULT_HOST = "hivegame.com"
AUTH_REFUSALS = ("Auth failed", "Too many Auth attempts", "Already authenticated")


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
        # A refused login is an HTTP error status, but its body still says why.
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
        self._buffered: list[dict[str, Any]] = []
        self._inbox: asyncio.Queue[Any] | None = None
        self._reader: asyncio.Task[None] | None = None

    async def __aenter__(self) -> HiveBot:
        await self.connect()
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()

    async def connect(self) -> dict[str, Any]:
        """Returns your own account: `username`, `uid` and your ratings.

        There is no other way to learn your own uid, and you need it to tell which colour
        you are. Nothing is pushed after this; ask for your games with `get_ongoing_games`
        or `get_pending_games`.
        """
        # A game list arrives as one frame and runs to megabytes for a bot with many games;
        # `websockets` closes the connection (1009) on anything over 1 MiB by default.
        self._socket = await websockets.connect(self._url, max_size=None)
        self._buffered = []
        self._inbox = asyncio.Queue()
        self._reader = asyncio.create_task(self._read_forever(self._socket, self._inbox))
        try:
            await self.send({"Auth": self._token})
            while True:
                message = await self._next()
                if "Authenticated" in message:
                    return message["Authenticated"]
                if message.get("Error") in AUTH_REFUSALS:
                    raise HiveError(f"authentication failed: {message['Error']}")
                # A connection is subscribed to the lobby before it authenticates, so anything
                # broadcast in the meantime — including our own Online — is traffic, not an answer.
                self._buffered.append(message)
        except BaseException:
            # `__aenter__` is this method, so a raise here never reaches `__aexit__`, and the
            # server leaves the socket open after refusing a token rather than hanging up.
            # Without this a retry loop on a bad token leaks a connection per attempt.
            await self.close()
            raise

    async def close(self) -> None:
        if self._reader is not None:
            self._reader.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._reader
            self._reader = None
        if self._inbox is not None:
            self._inbox.put_nowait(HiveError("not connected"))
            self._inbox = None
        if self._socket is not None:
            await self._socket.close()
            self._socket = None
        self._buffered = []

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
        """Your games where it is your turn."""
        await self.send("GetPendingGames")

    async def get_ongoing_games(self) -> None:
        """All your unfinished games, including the ones waiting on your opponent."""
        await self.send("GetOngoingGames")

    async def get_user(self, user_id: str) -> None:
        await self.send({"GetUser": user_id})

    async def get_username(self, username: str) -> None:
        await self.send({"GetUsername": username})

    async def resync(self) -> None:
        await self.send("Resync")

    async def _receive(self, socket: Any) -> dict[str, Any]:
        envelope = json.loads(await socket.recv())
        # A refusal is yielded like any other message, so one rejected move cannot end the
        # bot's message loop.
        if "Err" in envelope:
            return {"Err": envelope["Err"]}
        return envelope["Ok"]

    async def _read_forever(self, socket: Any, inbox: asyncio.Queue[Any]) -> None:
        # A bot awaiting its engine is not asking for messages. Without a reader of its own,
        # `websockets` stops reading once 16 frames queue up, the connection's pings go
        # unanswered and the server hangs up; and a latency ping answered late counts as lag.
        try:
            while True:
                message = await self._receive(socket)
                ping = message.get("Ping")
                if ping is None:
                    inbox.put_nowait(message)
                else:
                    await socket.send(json.dumps({"Pong": ping["nonce"]}))
        except Exception as error:
            inbox.put_nowait(error)

    async def _next(self) -> dict[str, Any]:
        inbox = self._inbox
        if inbox is None:
            raise HiveError("not connected")
        message = await inbox.get()
        if isinstance(message, Exception):
            # Left in place so every later read fails the same way instead of waiting forever.
            inbox.put_nowait(message)
            raise message
        return message

    async def messages(self) -> AsyncIterator[dict[str, Any]]:
        """Yield server messages forever, starting with anything that arrived during `connect`."""
        while self._buffered:
            yield self._buffered.pop(0)
        while True:
            yield await self._next()
