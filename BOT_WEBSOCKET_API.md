# Bot WebSocket API

A bot can play moves, manage challenges, and read games and users over a single WebSocket
instead of repeated HTTP requests. The server pushes updates, so a bot no longer polls
`GET /api/v1/bot/games/pending` to find out that the opponent moved.

The HTTP bot API still works and is unchanged. You can use both.

## Getting a token

This still happens over HTTP and has not changed. It is the only HTTP request a WebSocket bot
needs:

```
POST /api/v1/auth/token
{ "email": "bot@example.com", "password": "..." }

→ { "success": true, "data": { "token": "<jwt>" } }
```

The account must be flagged as a bot; the endpoint refuses anyone else.

## Connecting

Open a WebSocket to `/ws/?format=json` (`wss://hivegame.com/ws/?format=json`). The
`format=json` matters: without it the server replies in MessagePack, which is what the browser
client speaks.

There is no cookie or auth header. The connection starts anonymous, and you authenticate it by
sending this as the first frame:

```
{ "Auth": "<jwt>" }
```

On success the server replies `{"Ok": {"AuthOk": {"username": "<you>"}}}` and then sends a
`LobbySnapshot`. On failure it replies `{"Ok": {"Error": "Auth failed"}}` and the socket stays
anonymous; send another `Auth` frame to retry. Once a socket is authenticated a further `Auth` is
refused with `{"Ok": {"Error": "Already authenticated"}}` — reconnect to change account.

**Wait for `AuthOk`, not for the snapshot.** The server starts writing the moment it accepts the
connection and sends a latency `Ping` to every socket once a second, so frames arrive before your
`Auth` is even read — including a `LobbySnapshot` for the anonymous connection. `AuthOk` is the
only frame that proves the token was accepted. Read past anything else until you see it.

While the socket is anonymous, every request that acts for you is refused: playing, game
controls, challenges, and all the reads below.

## Frame encoding

Send JSON in text frames; you receive JSON in text frames. Every request is one frame.

Requests are a tagged union with two shapes:

| shape | JSON | example |
|---|---|---|
| variant with a payload | object with the variant name as its single key | `{"Auth": "tok"}` |
| variant without a payload | the name as a bare string | `"GetPendingGames"` |

```json
{"Auth": "tok"}
{"GetGame": "abc"}
"GetPendingGames"
{"Game": {"game_id": "abc", "action": {"Play": "wA1 -bQ"}}}
```

The variant names are the contract. Renaming one is a breaking change and we treat it as one;
their order in our source means nothing, and no index is involved.

Every server message arrives wrapped in a result envelope:

```json
{"Ok":  { ...message... }}
{"Err": { ...error... }}
```

The browser client talks to the same endpoint in MessagePack, because it shares our Rust types
and gets the encoding for free. You do not need to care, and you should not try to match it —
`?format=json` exists so you never have to.

## Requests

### Playing

| what | frame |
|---|---|
| play a move | `{"Game": {"game_id": "<nanoid>", "action": {"Play": "wA1 -bQ"}}}` |
| resign, offer/accept draw, takeback | `{"Game": {"game_id": "<nanoid>", "action": {"Control": {"Resign": "White"}}}}` |
| subscribe to a game's updates | `{"Game": {"game_id": "<nanoid>", "action": "Join"}}` |
| stop receiving them | `{"Game": {"game_id": "<nanoid>", "action": "Unwatch"}}` |

Moves use UHP notation, the same string the `piece_pos` field takes in
`POST /api/v1/bot/games/play`. The server resolves it against the board it replays. On the first
move of a game, send just the piece (`"wA1"`).

The browser client sends a structured `Turn` action instead. Do not copy it. The browser runs a
copy of the engine in WebAssembly, so it knows the board coordinates; a bot does not, and those
coordinates move when the board recenters.

A game control is its name wrapping your colour. The names are `Resign`, `DrawOffer`,
`DrawAccept`, `DrawReject`, `TakebackRequest`, `TakebackAccept`, `TakebackReject` and `Abort`,
and the colour is `"White"` or `"Black"`.

`Join` also announces you to the other participants. To look at a game without joining it,
use `GetGame` below.

### Challenges

| what | frame |
|---|---|
| create | `{"Challenge": {"Create": <challenge details>}}` |
| accept | `{"Challenge": {"Accept": "<challenge nanoid>"}}` |
| delete one | `{"Challenge": {"Delete": "<challenge nanoid>"}}` |
| delete several | `{"Challenge": {"DeleteMany": ["<nanoid>", ...]}}` |

### Reads

| what | frame | replaces |
|---|---|---|
| any game by id | `{"GetGame": "<nanoid>"}` | `GET /api/v1/bot/game/{nanoid}` |
| all your unfinished games | `"GetOngoingGames"` | `GET /api/v1/bot/games/ongoing` |
| your games awaiting action | `"GetPendingGames"` | `GET /api/v1/bot/games/pending` |
| a user by uuid | `{"GetUser": "<uuid>"}` | `GET /api/v1/bot/user/{id}` |
| a user by username | `{"GetUsername": "<username>"}` | — |
| re-send the lobby snapshot | `"Resync"` | `GET /api/v1/bot/games/ongoing`, `GET /api/v1/bot/challenges/` |

`GetGame` works on any game, not just your own, and does not subscribe you to anything. It is
the read-only version of `Join`.

## Server messages

What a bot will actually receive:

| message | when |
|---|---|
| `{"AuthOk": {"username": "..."}}` | your `Auth` was accepted. The only proof of it |
| `{"LobbySnapshot": {...}}` | on connect, after `Auth`, and on `Resync`. Its `urgent_games` holds the games **awaiting your move**, not every ongoing game |
| `{"Ping": {"nonce": N, "value": F}}` | once a second, to every socket. Answer with `{"Pong": N}` or your reported latency stays at zero |
| `{"Game": {"Reaction": {...}}}` | a game you are in changed — this is the move notification that replaces polling |
| `{"Game": {"Urgent": [...]}}` | games needing your action; also the reply to `GetPendingGames` |
| `{"Game": {"Ongoing": [...]}}` | reply to `GetOngoingGames`: every unfinished game you play, whoever is to move |
| `{"Game": {"Fetched": {...}}}` | reply to `GetGame` |
| `{"UserProfile": {...}}` | reply to `GetUser` / `GetUsername` |
| `{"Challenge": {...}}` | a challenge was created, accepted or removed |
| `{"Error": "..."}` | something was refused |

## Session behaviour

**Heartbeats.** The server pings every 5 seconds and drops the connection if it has heard
nothing for 10. Most WebSocket libraries answer pings for you; check that yours does.

**Token expiry does not apply once you are connected.** The JWT lasts 100 minutes, and the
server checks that when you send `Auth`, so an expired token is refused. It does not re-check an
open socket, which stays authenticated until it closes. The browser works the same way: its
session cookie is only checked when the socket opens.

**Deleting the account cuts every socket immediately.** That check is per request, not per
connection.

**Reconnecting.** A reconnect keeps nothing. Authenticate again, then `Join` any game you want
updates for. The `LobbySnapshot` sent after `Auth` carries only the games waiting on your move,
so send `GetOngoingGames` to enumerate the rest — a game where you are waiting on your opponent
appears in neither the snapshot nor `GetPendingGames`.

## Still HTTP only

Fetching a token. Every other bot operation has a frame above.
## Client libraries

There is one reference client, in Python. It is deliberately not written in Rust.

A Rust client would `use` the server's own `ClientRequest` and `ServerMessage` types and share
its codec, so it would be correct by construction — which is exactly what makes it useless as a
reference. It would never build a frame by hand, never touch the JSON transport, and never show
you anything you need if you cannot depend on our crates.

The Python client speaks JSON over a plain WebSocket and takes every shape from this document,
so it stands where you stand. Read it, translate it into your language, or hand it and this page
to an AI and have it translated for you.

**If you are writing a bot in Rust, you have a better option than any client library.** Depend on
the `apis` crate and use `ClientRequest` and `ServerMessage` directly over msgpack. The compiler
then guarantees your bot matches the server, and you skip this document entirely. Nothing in this
repo does that yet — `hive-hydra` is a REST client and speaks none of the above.
