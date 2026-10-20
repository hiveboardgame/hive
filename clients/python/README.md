# hivegame bot client (Python)

One file, one dependency. Copy `hivegame_bot.py` into your project.

```
pip install websockets
```

```python
import asyncio
from hivegame_bot import HiveBot, fetch_token

async def main():
    token = fetch_token("bot@example.com", "hunter2")
    async with HiveBot(token) as bot:
        await bot.get_ongoing_games()
        async for message in bot.messages():
            if "Err" in message:
                print("refused:", message["Err"])
                continue
            game = message.get("Game", {}).get("Reaction")
            if game is None:
                continue
            # your engine decides; then:
            # await bot.play(game["game_id"], "wA1 -bQ", turn=game["game"]["turn"])

asyncio.run(main())
```

`HiveBot.connect` authenticates and returns your own account: `username`, `uid` and your
ratings. The `uid` is how you tell which colour you are, and nothing else in the API will tell
you it. Nothing is pushed after authenticating, so ask for your games next with
`bot.get_ongoing_games()` (all of them) or `bot.get_pending_games()` (your turn). The answer
comes on the message stream, after anything that arrived while you were authenticating.

The socket is read in a background task, so the connection stays up and latency pings are
answered on time while your loop awaits its engine. That needs a free event loop: run a
synchronous engine with `await asyncio.to_thread(...)` or in a subprocess, never inline, or
nothing is read until it returns and the server hangs up.

A refused request (an illegal move, a challenge that breaks the rules) is yielded on the stream
as `{"Err": {...}}`, never raised, so one bad move cannot end your loop. `Err.Game` carries the
`game_id` it was about.

Every method is a one-line wrapper over a frame in
[`BOT_WEBSOCKET_API.md`](../../BOT_WEBSOCKET_API.md). If you need something not wrapped here,
`bot.send(frame)` takes the raw structure.

## Porting this to another language

That is the intent. The client is deliberately small and speaks plain JSON over a plain
WebSocket, so it reads as a specification you can translate. Hand this file and
`BOT_WEBSOCKET_API.md` to an LLM and ask for the same thing in your language; the result is
checkable against the frame tables in the doc.

That includes Rust. There is no Rust client to depend on: the server's crates are a web
application, not a bot library, and `BOT_WEBSOCKET_API.md` is the contract.

## Tests

`test_hivegame_bot.py` checks that every method builds the frame the documentation says it
does, against a stub socket. No server needed:

```
python3 test_hivegame_bot.py
```

The frame shapes it asserts are the same ones pinned on the Rust side in
`apis/src/common/client_message.rs`, so the two cannot drift apart quietly.

## Status

The frames are verified; the client has not yet been run against a live server. If you are the
first to do that, expect to fix something small, and please say what.
