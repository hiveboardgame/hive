# Tournament seeding scripts

Builds a database full of tournaments in every mode and every state, so the
frontend can be clicked through with real data in it.

```sh
cargo run --bin script tournaments              # seed
cargo run --bin script tournaments --play-moves # seed, with real boards
cargo run --bin script cleanup                  # remove it all again
```

`DATABASE_URL` must be set, or pass `--database-url`. Point it at a development
database — this writes a lot of rows.

## What it creates

Seventeen users: `tt-01` .. `tt-16` as players, and `tt-org` as the organizer of
everything. Password for all of them is `hivegame`. Log in as `tt-org` for the
organizer controls, or as `tt-01` for a participant's view.

Ratings descend from 2000 in steps of 10, so seeding is deterministic and seed
order is legible in the standings rather than falling out of uuid order.

Then, for each of `SingleRoundRobin`, `DoubleRoundRobin`, `DutchSwiss`,
`BursteinSwiss`, `DoubleSwiss`, `SingleElimination` and `DoubleElimination`,
three tournaments:

- **done** — played out and finished.
- **live** — a round or two played, then left mid-event. This is what shows
  mid-tournament standings and a half-filled bracket.
- **upcoming** — not started. One per mode is scheduled (`StartMode::Date`, six
  hours out) so the countdown renders, and one is invite-only with the field
  invited and unanswered, for the organizer panel's pending list.

Arena gets two instead, because a not-yet-open arena shows nothing: one
**running** for three hours with games still open and seats to spare — what the
front-page card and its join button need — and one **finished**.

Field sizes vary by mode rather than being a flat sixteen. `replay()` is
O(games) per read and runs once per `progress()`, so a large field in a
repeating format costs quadratic time for no extra coverage.

Re-running is safe: users are found by name and reused. Tournaments are always
new, with a nanoid suffix, because `tournaments.name` is unique.

## Notes

Results are decided rather than played by default — `--play-moves` adds a dozen
real legal moves per game first, which is slower but gives every game a board
worth opening.

In the two-game-match modes (`DoubleSwiss`, both eliminations) results follow a
scripted better-seed-wins rule. A 1-1 match there is *unresolved*, not drawn,
and triggers a replay, so random per-game results would frequently fail to
resolve a match. The single-game modes take random results, draws included.

Arena finishes are stamped explicitly, 100ms apart and measured from each game's
own `created_at`, with a pause between ticks. An arena replays from a timeline of
instants at millisecond resolution: a finish sharing an instant with a pairing,
or landing after the *next* pairing, reorders that timeline — which replay treats
as fatal and cannot recover from.

## Fixtures for the browser tests

`tournaments` writes `apis/end2end/seeded.json` — mode, stage, name and nanoid
for everything it created. The Playwright suite reads it as fixtures, because
names and nanoids are fresh every run (`tournaments.name` is unique) so no test
can hardcode a URL. `--no-manifest` skips it, `--manifest <path>` moves it.

Full loop:

```sh
cargo run --bin script cleanup
cargo run --bin script tournaments
cargo leptos serve                 # in another shell
cargo leptos end-to-end
```
