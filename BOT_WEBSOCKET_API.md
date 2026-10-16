# Bot WebSocket API

A bot can play moves, manage challenges, and read games and users over a single WebSocket
instead of repeated HTTP requests. The server pushes updates, so a bot no longer polls
`GET /api/v1/bot/games/pending` to find out that the opponent moved.

The socket also gives a bot more than the HTTP bot API does: chat, tournaments, schedules and
the rest of what a signed-in player can do. See [Beyond the HTTP API](#beyond-the-http-api).

The HTTP bot API still works and is unchanged. You can use both.

## Getting a token

This still happens over HTTP and has not changed. It is the only HTTP request a WebSocket bot
needs:

```
POST /api/v1/auth/token
{ "email": "bot@example.com", "password": "..." }

→ { "success": true, "data": { "token": "<jwt>" } }
```

The account must be flagged as a bot; the endpoint refuses anyone else. A refused login answers
with HTTP 400 and `{"success": false, "data": {"message": "..."}}`.

## Connecting

Open a WebSocket to `/ws/?format=json` (`wss://hivegame.com/ws/?format=json`). The
`format=json` matters: without it the server replies in MessagePack, which is what the browser
client speaks.

Send no cookie, and no `Origin` or `Referer` header. Browsers must send an `Origin` that matches
the site, so a handshake that carries a different one is refused with HTTP 403. Most WebSocket
libraries send no `Origin` unless you set one.

The connection starts anonymous, and you authenticate it by sending this as the first frame:

```
{ "Auth": "<jwt>" }
```

On success the server binds the socket to your bot account and replies
`{"Ok": {"Authenticated": {...your account...}}}`. No lobby snapshot is pushed, neither on
connect nor after `Auth`: ask for your games with `GetOngoingGames` or `GetPendingGames`, and for
the lobby with `Resync`. On failure it replies `{"Ok": {"Error": "Auth failed"}}`
and the socket stays anonymous.

`Authenticated` is the only frame that means your `Auth` landed, and it is how you learn your own
`uid`. Nothing else tells you, and you need it to tell which colour you are playing. Wait for it
rather than for the next message: the socket joins the lobby before it authenticates, so
broadcasts and pings can arrive in between.

A socket authenticates once. Another `Auth` on an authenticated socket is refused with
`{"Ok": {"Error": "Already authenticated"}}`. Failed attempts are limited to 3 per 10 seconds per
socket; past that the server answers `{"Ok": {"Error": "Too many Auth attempts"}}` without
checking the token.

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

A frame that is not valid JSON, or does not match any request, is answered with
`{"Err": {"Request": {"reason": "Malformed request: ..."}}}` naming what the parser rejected.

The browser client talks to the same endpoint in MessagePack, because it shares our Rust types
and gets the encoding for free. You do not need to care, and you should not try to match it.
`?format=json` exists so you never have to.

## Results and errors

Every server message arrives wrapped in a result envelope:

```json
{"Ok":  { ...message... }}
{"Err": { ...error... }}
```

A refused request comes back as an `Err`:

| error | when |
|---|---|
| `{"Err": {"Game": {"game_id": "<nanoid>", "reason": "..."}}}` | a `Game` request (move, control, join, …) was refused |
| `{"Err": {"Request": {"reason": "..."}}}` | any other request was refused |
| `{"Err": {"Unauthorized": {"reason": "..."}}}` | you are not allowed to do that, usually because the socket is not authenticated |

The `reason` names what went wrong when you can act on it: an illegal move, a move when it is not
your turn, a game that is over or does not exist, notation the server cannot parse, a challenge
that breaks the rules below. When the failure is on our side the reason is
`"Unable to complete request"`, and retrying later is the only fix.

A few refusals arrive as an ordinary message instead, `{"Ok": {"Error": "..."}}`: authentication
(above), accepting a challenge outside its rating band, and challenges `DeleteMany` could not
delete.

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
and the colour is `"White"` or `"Black"`. The HTTP API only offers resign and abort; here you get
all of them. Tournament games allow neither takebacks nor aborts.

You receive every change to your own games without joining them. `Join` subscribes you to a game
you are watching, and also announces you to the other participants. To look at a game without
joining it, use `GetGame` below.

### Challenges

| what | frame |
|---|---|
| create | `{"Challenge": {"Create": <challenge details>}}` |
| accept | `{"Challenge": {"Accept": "<challenge nanoid>"}}` |
| delete one | `{"Challenge": {"Delete": "<challenge nanoid>"}}` |
| delete several | `{"Challenge": {"DeleteMany": ["<nanoid>", ...]}}` |

Challenge details:

```json
{"rated": true, "game_type": "MLP", "visibility": "Public", "opponent": null,
 "color_choice": "Random", "time_mode": "RealTime", "time_base": 300, "time_increment": 3,
 "band_upper": null, "band_lower": null}
```

Times are in seconds. A `Correspondence` challenge sets exactly one of `time_base` (total time
each) or `time_increment` (time per move); an `Untimed` one sets neither. `visibility` is
`Public`, `Private` or `Direct`, and only a `Direct` challenge names an `opponent` (a username).

Every challenge, however it is created, must follow these rules:

- `game_type` is `Base` or `MLP`.
- A rated game is `MLP`, and not `Untimed`.
- `band_lower` is not above `band_upper`.

The HTTP API also limits the clock: real time is 1–180 minutes plus 0–180 seconds, and
correspondence is 1–20 days. The socket accepts wider clocks, because a rematch inherits the
clock of the game it follows, and tournament games can be longer.

`DeleteMany` deletes what it can. Each removal is announced as usual, and the ones it could not
delete are listed in one `{"Ok": {"Error": "Could not delete challenges: ..."}}`.

### Reads

| what | frame | answer | replaces |
|---|---|---|---|
| any game by id | `{"GetGame": "<nanoid>"}` | `Fetched` | `GET /api/v1/bot/game/{nanoid}` |
| your games where it is your turn | `"GetPendingGames"` | `Urgent` | `GET /api/v1/bot/games/pending` |
| all your unfinished games | `"GetOngoingGames"` | `Ongoing` | `GET /api/v1/bot/games/ongoing` |
| a user by uuid | `{"GetUser": "<uuid>"}` | `UserProfile` | `GET /api/v1/bot/user/{id}` |
| a user by username | `{"GetUsername": "<username>"}` | `UserProfile` | — |
| the lobby snapshot | `"Resync"` | `LobbySnapshot` | `GET /api/v1/bot/challenges/` |

Games come as the same game objects the browser uses, not the raw rows the HTTP API returns.

`GetGame` works on any game, not just your own, and does not subscribe you to anything. It is
the read-only version of `Join`.

`GetPendingGames` and `GetOngoingGames` leave out games whose clock has already run out. The
server has not finalized those yet, but they are lost on time and no move can be played in them.

The lobby snapshot carries the games where it is your turn, every challenge you can see (open
ones, yours, and the ones sent to you), the games on TV and who is online. It is not a list of
your games: use `GetOngoingGames` for that.

## Beyond the HTTP API

An authenticated bot socket can do what a signed-in player can. These requests have no HTTP
equivalent:

| what | frame |
|---|---|
| send a chat message | `{"Chat": {"key": <conversation>, "client_id": "<uuid you choose>", "body": "gg", "turn": null}}` |
| subscribe to a conversation | `{"ChatSubscribe": {"key": <conversation>, "session_epoch": 0, "request_id": 0}}` |
| unsubscribe | `{"ChatUnsubscribe": <conversation>}` |
| join a tournament | `{"Tournament": {"Join": "<tournament nanoid>"}}` |
| leave it | `{"Tournament": {"Leave": "<tournament nanoid>"}}` |
| join an arena | `{"Tournament": {"ArenaJoin": "<tournament nanoid>"}}` |
| pause or resume in an arena | `{"Tournament": {"ArenaPause": "<id>"}}`, `{"Tournament": {"ArenaResume": "<id>"}}` |
| follow a tournament's updates | `{"TournamentWatch": "<tournament nanoid>"}`, `{"TournamentUnwatch": "<id>"}` |
| berserk in an arena game | `{"Game": {"game_id": "<nanoid>", "action": "Berserk"}}` |
| ready up for a scheduled game | `{"Game": {"game_id": "<nanoid>", "action": "Start"}}` |
| mark a game's notification seen | `{"NotificationSeen": {"game_id": "<nanoid>"}}` |
| show yourself as away | `"Away"` |

A conversation is one of:

```json
"Global"
{"Direct": "<user uuid>"}
{"Tournament": "<tournament nanoid>"}
{"Game": {"game_id": "<nanoid>", "thread": "players"}}
{"Game": {"game_id": "<nanoid>", "thread": "spectators"}}
```

`client_id` is yours to generate, one per message: send the same one again only to retry the same
message. Chat follows the same rules as for people: the global channel is admin-only, players
cannot write to spectators while their game runs, and sending is rate limited.

Tournament organizing (`Create`, invitations, `Start`, `Kick`, …) and schedule proposals
(`Schedule`) are also available. Their payloads are the browser's own; ask before you build on
them, because unlike the frames above they are not yet pinned by tests and may change.

## Server messages

What a bot will receive:

| message | when |
|---|---|
| `{"Authenticated": {...}}` | your `Auth` succeeded. Your own account, including the `uid` you play as |
| `{"Game": {"Reaction": {...}}}` | a game you are in changed. This is the move notification that replaces polling |
| `{"Game": {"Urgent": [...]}}` | games where it is your turn; the reply to `GetPendingGames`, and pushed when that list changes |
| `{"Game": {"Ongoing": [...]}}` | reply to `GetOngoingGames` |
| `{"Game": {"Fetched": {...}}}` | reply to `GetGame` |
| `{"Game": {"Heartbeat": {...}}}` | clock times of a game you joined, every few seconds while it runs |
| `{"Game": {"Tv": {...}}}` | a game on TV changed |
| `{"Game": {"OwnGameRemoved": "<nanoid>"}}` | one of your games left your list (finished or deleted) |
| `{"UserProfile": {...}}` | reply to `GetUser` / `GetUsername` |
| `{"LobbySnapshot": {...}}` | reply to `Resync` |
| `{"Challenge": {...}}` | a challenge was created, accepted or removed |
| `{"UserStatus": {...}}` | someone came online, went offline or away |
| `{"Join": "<uuid>"}` | someone joined a game you joined |
| `{"Chat": {...}}` | a chat message in a conversation you are in or subscribed to |
| `{"Tournament": {...}}`, `{"Schedule": {...}}` | updates for tournaments and schedules you take part in or watch |
| `{"Ping": {"nonce": 123, "value": 42.0}}` | answer with `{"Pong": 123}` (see below) |
| `{"Error": "..."}` | an ordinary refusal (see [Results and errors](#results-and-errors)) |

Ignore any message you do not recognize. New ones are added without notice.

## Session behaviour

**Heartbeats.** The server sends a WebSocket ping every 5 seconds and drops the connection if it
has heard nothing for 10. Most WebSocket libraries answer pings for you; check that yours does.

**Lag compensation.** Separately, the server sends `{"Ping": {"nonce": n, "value": ms}}` every
second. Answer each one with `{"Pong": n}`. The server times the round trip and credits
your real-time clock with the measured network lag on each move, up to a small per-game quota.
`value` is your current measured lag in milliseconds. If you do not answer, you get no
compensation. Answer promptly: compensation is capped per game, and how lag is measured may
change.

**Token expiry does not apply once you are connected.** The JWT lasts 100 minutes, and the
server checks that when you send `Auth`, so an expired token is refused. It does not re-check an
open socket, which stays authenticated until it closes. The browser works the same way: its
session cookie is only checked when the socket opens.

**Deleting the account stops the bot immediately, but does not close the socket.** Every request
is refused from then on, and apart from those refusals nothing more is sent. Close it yourself.

**Reconnecting.** A reconnect keeps nothing. Authenticate again, ask for `GetOngoingGames`, and
`Join` any game you want to watch.

## Still HTTP only

Fetching a token. Every other bot operation has a frame above.

## Client libraries

There is one reference client, in Python, in `clients/python/`. It speaks JSON over a plain
WebSocket and takes every shape from this document, so it stands where you stand. Read it,
translate it into your language, or hand it and this page to an AI and have it translated for
you.

There is no Rust client, and Rust bots speak this JSON protocol too. The server's Rust crates are
a web application, not a library for bots, and they change without regard for you; this document
is the contract.
