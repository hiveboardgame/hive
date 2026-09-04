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
        async for message in bot.messages():
            game = message.get("Game", {}).get("Reaction")
            if game is None:
                continue
            # your engine decides; then:
            # await bot.play(game_id, "wA1 -bQ")

asyncio.run(main())
```

`HiveBot.connect` returns the lobby snapshot, which lists the games and challenges you already
have — enough to rebuild your state after a restart.

Every method is a one-line wrapper over a frame in
[`BOT_WEBSOCKET_API.md`](../../BOT_WEBSOCKET_API.md). If you need something not wrapped here,
`bot.send(frame)` takes the raw structure.

## Porting this to another language

That is the intent. The client is deliberately small and speaks plain JSON over a plain
WebSocket, so it reads as a specification you can translate. Hand this file and
`BOT_WEBSOCKET_API.md` to an LLM and ask for the same thing in your language; the result is
checkable against the frame tables in the doc.

**If your bot is in Rust, do not port this.** Depend on the `apis` crate and use `ClientRequest`
and `ServerMessage` directly over MessagePack. The compiler then guarantees your bot matches the
server, and you can ignore the wire format entirely. `hive-hydra` in this repo is the example.

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
