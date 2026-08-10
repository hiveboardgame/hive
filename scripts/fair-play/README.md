# Fair-play tooling

Offline analysis for detecting one game's positions being reproduced in another
game used as an oracle — the *"play the bot alongside your real game and copy its
moves"* pattern.

Nothing here runs on the server, nothing here decides anything, and nothing here
can sanction an account. It reads finished games, produces review files for a
human, and records what humans decided.

## Why this works at all

hivegame.com already computes a **canonical position hash** for every position of
every finished game (`engine/src/hasher.rs`, stored in `game_hashes`). It is
invariant to board translation, the six rotations, reflection, move-order
transposition and interchangeable piece labels (`wA1` vs `wA2`). Two games sharing
a deep run of positions are therefore playing *the same game*, not a similar one —
and no cheap move-order trick escapes it.

Caveat inherited from the same hash: **`game_type` is not part of it**, so Base and
Base+MLP positions collide. Every comparison filters on `game_type`, exactly as the
explorer's own queries do.

## Running it

```bash
# 1. build a corpus (or export equivalent fields straight from the database)
py -3 scripts/fair-play/pull_archive.py --out archive.jsonl --delay 1.0

# 2. scan it, writing one graded review file per implicated account.
#    Default alpha is 1e-3 (precision-first). Add --alpha 0.01 for a wider research
#    sweep, and --enable-seat to turn the (opt-in, off-by-default) seat inference on.
py -3 scripts/fair-play/mirror_scan.py --archive archive.jsonl --out logs/fair-play

# 3. same scan, plus the empirical false-positive baseline (see below). Slower.
py -3 scripts/fair-play/mirror_scan.py --archive archive.jsonl --out logs/fair-play \
       --null-permutations 500

# 4. ranked list of accounts with the reason for each
py -3 scripts/fair-play/suspects.py --archive archive.jsonl

# 5. the same scan, also recording each surviving pair in the durable registry.
#    Every implicated account lands on `suspicious` and nothing higher, ever.
py -3 scripts/fair-play/mirror_scan.py --archive archive.jsonl --out logs/fair-play \
       --registry

# 6. the durable record
py -3 scripts/fair-play/registry.py list
py -3 scripts/fair-play/registry.py list --recheck
py -3 scripts/fair-play/registry.py label <uid> proven_cheater --by <you> --why "..."

# 7. opening-explorer use past ply 8 — a suspicion mark, never evidence, never
#    written to the registry
py -3 scripts/fair-play/book_follow.py --archive archive.jsonl --top 25

# 8. SECOND STAGE ONLY, on an account the position evidence already selected.
#    Needs a GPU eval server — read the warning in the module docstring before
#    starting one on a host that also serves the seated bot.
py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --calibrate --games 120
py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --uid <uid>
```

`--registry` writes to `logs/cheat/registry.json` (gitignored, like everything under
`logs/`). It attaches evidence to exactly the accounts the case files are written for
— an account in *both* games, or the human seat of an unattributable pair's *oracle*
game — and to nobody else. A pure opponent, an unattributable pair's real-game player
and any account flagged `bot` are all excluded, and the attribution is not re-derived
there: it reuses the mapping the case files come from, because a rule stated twice is
a rule that drifts.

Re-running the scan is idempotent — verified: two identical runs at the shipped default
leave 41 signals and 41 log entries, 0 accounts marked for recheck. The de-duplication
key is `(code, detail)`,
so `detail` carries only the unordered pair of game ids — never bits, position counts or
the grade, all of which move as the corpus grows.

`code` is a short stable slug (`mirror.self`, `mirror.oracle_side`,
`mirror.seat_inference`, `linked.bot_games`) and `signal` is the sentence a moderator
reads. They were the same field, and that was a live hazard: the sentences are long
explanatory prose, a new signal counts as new evidence, and new evidence on a
human-labelled account sets `needs_recheck` and logs "new evidence arrived after human
review". Fixing a typo in one of those sentences therefore invalidated every volunteer's
clearance at once and refilled the recheck queue with findings they had already
dismissed. **A proof-reading pass is not new evidence.** Keep the codes stable; reword
the sentences freely.

`pull_archive.py` scrapes the public archive politely (one request per second,
keyset-friendly splitting, resumable). If you have database access you do not need
it — export `game_id`, both player uids and bot flags, both ratings and rating
changes, `speed`, `rated`, `game_type`, `tournament`, `created_at`,
**`last_interaction`**, `updated_at`, `conclusion`, `history`, `hashes`, `move_times`.

Output goes under `logs/`, which is gitignored. **Keep it that way** — review files
contain usernames.

## Failure posture

Every ambiguity resolves *against* raising a case, and every drop is counted and
printed rather than being silent. Pairs are discarded — not admitted — when the
end-of-game timestamp cannot be trusted, when a duration is implausible for its
speed, when the real game was unrated, when roles cannot be determined, and when
the shared positions belong to a cluster rather than to a pair. The detector is
built to lose recall rather than to manufacture an accusation.

## Labels

    unreviewed      nobody has looked
    suspicious      evidence exists, no human conclusion yet
    normal          a human looked and cleared the account — an explicit clearance
    proven_cheater  a human concluded, with a written reason on the record

The detector can only ever reach `suspicious`. `normal` and `proven_cheater` both
require a named human and a reason of at least 20 characters, enforced in code.
That is the project's own "never auto-ban" rule made structural, and it is also the
cleanest posture under GDPR Art. 22 (no solely-automated decisions with a
significant effect on a person).

Automation never overwrites a human label — but new evidence arriving after a human
decided is **logged and flagged** (`needs_recheck`, surfaced by `list --recheck` and
marked `stale` in `export-labels`). It used to be swallowed entirely, which made a
clearance permanent and unconditional and quietly poisoned the label set that any
future classifier would train on.

## Turning things off

Every output of this tooling is a flag a human can switch off. There is no exception,
and the off switches are layered so the response can match the size of the problem:

| scope | mechanism | survives rescans? |
|---|---|---|
| one finding | `registry.py dismiss <uid> --code <c> --detail <d> --by <you> --why "..."` | yes — the `(code, detail)` identity means the weekly scan lands on the same record and `is_new` stays false |
| one account | `registry.py label <uid> normal --by <you> --why "..."` | yes — automation never overwrites a human label; genuinely new evidence sets `needs_recheck` instead |
| the seat inference | **off by default** (opt in with `mirror_scan.py --enable-seat`, likewise `suspects.py --enable-seat`) | per run — the scan prints its config line |
| the linked / self-mirror families | `mirror_scan.py --disable linked` / `--disable self-mirror` | per run — the scan prints its config line so no two runs are silently incomparable |
| the explorer mark | nothing to turn off — `book_follow.py` never writes to the registry at all | — |
| the engine check | on-demand only; it has no standing output to disable | — |

Both registry-level switches are **human-only** (a name that is not `detector` after
trimming, a reason of at least 20 characters after trimming) and **append-only
logged** — a dismissal is a decision on the record, not a deletion. Dismissing one
finding never mutes the next: a different pair of games is a different `detail`, which
is a new signal, which surfaces normally.

The seat inference is **off by default** (external review D1): the colour/parity
mapping is sound, but "this seat corresponds to the bot's colour" → "this seat received
the bot's advice" assumes the oracle was advising rather than analysing/predicting the
opponent, and its 15/15 validation is one operation seen 15 times. With it off, both
real-game names are withheld on every alt-account pair. The switch sits inside
`seat_correspondence()` itself, so attribution, case files, `suspects.py` and the
registry all follow it together — no consumer has its own copy of the rule.
The relay core has no switch on purpose: if the position evidence itself is wrong,
the answer is to stop running the tool, not to run it with its heart cut out.

## Evidence grades

| grade | meaning |
|---|---|
| `p1` | one unbroken run of ≥40 positions against a bot, oracle opened in the first 15%, same account in both games |
| `p2` | same account in both games, bot oracle, weaker on run length or timing |
| `p3` | the oracle sits on a *different* account, or the oracle is human — needs a database lookup to attribute |

Grades set queue order. They decide nothing.

**There is deliberately no grade called `conclusive`.** Using a bot game as a live
analysis board and relaying moves out of one leave *identical* traces in every field
the archive records; the difference is intent, which is not in this data. A tier
named for certainty invites a reader to skip the caveat saying we do not have it.
A genuinely conclusive bar needs per-move causality — each oracle reply preceding
the corresponding real move, over a long run, with a stable lag — which needs
server-side per-ply timestamps, not the archive.

## False-positive controls

Each of these fired on real data during development:

- opening plies (< 12) excluded — everyone plays the same openings
- **one popularity threshold**, `MAX_FANOUT`, applied in exactly one place. There
  used to be two, and the scored set was a superset of the detected set: a pair whose
  real evidence was three rare positions could be reported as "43 positions
  reproduced against a bot"
- **a player's own repertoire** excluded — a pet line played in two concurrent games
  is the single largest false-positive family. Only games that *finished before* the
  real game started count, only the **subject's** games count, and the pair's own two
  games do not. Previously it was an untimed corpus-wide count over both games'
  players, so one game played a year later retroactively erased the evidence, and a
  victim's pet defence exonerated whoever was relaying against them.
  Where there *is* no subject — the 15 unattributable pairs, i.e. the majority — the
  exemption is inherited by the **oracle game's human seat**, whose own habit is what
  would explain that game. It used to be inherited by nobody: `for uid in subject_of(…)`
  simply never ran, so the exemption was silently off for most findings while this file
  and the case files both said it applied. Extending it to the real game's two players
  as well was measured and **rejected** — it costs nothing today either, and it hands
  anyone reading this a way out: pick opponents with a known pet defence, steer into it,
  and the exemption fires on the *victim's* history. An exemption the accused does not
  own is not an exemption. Measured: 0 of 21 survivors lose a position to the version
  that shipped
- **position clusters** excluded — a relay is an isolated two-game component; one
  line shared by *k* games is a clique of C(k,2) pairs
- rematches between the same two players excluded
- same tournament excluded, compared **by tournament id** — comparing the embedded
  `TournamentAbstractResponse` objects compared mutable counters captured at scrape
  time, so the exemption silently never fired
- bot-vs-bot excluded (out of scope by design)
- **live-window test** — the oracle must open early in the game and run inside it.
  A game replayed against a bot afterwards is analysis, not relaying, and is dropped
- **a human oracle must be a source of strength** — materially higher rated and not
  in a faster format
- **a bot game is never the game being played.** Roles used to be decided by start
  order for self-attributable pairs, so opening the analysis board 61 seconds early
  cast the rated game as a "human oracle" and erased the pair
- **an unknown speed class is dropped, not admitted.** Everything that reads `speed`
  used to default *open* — `MAX_PLAUSIBLE_HOURS.get` returned no cap and
  `SPEED_RANK.get(orac, 9) < SPEED_RANK.get(real, 0)` was never true — so a single
  unrecognised string switched off the duration quarantine and the faster-format test
  at once. `shared_types/src/game_speed.rs` has had a `Puzzle` variant in neither dict
  the whole time. Puzzles are out of scope, but the rule enforced is the general one
- **`created_at` is not always the start of play.** It is when the game *row* was made
  (`db/src/models/game.rs:182`). For a tournament game with `game_start = "Ready"` the
  row exists from tournament build time and play begins only once both players are
  ready, so `created_at` can precede the first move by days. That inflates the real
  game's duration, drives `start_pct` toward zero and `overlap_frac` toward one, and
  fails the live-window test **open** — the same shape as the `updated_at` bug on the
  other end of the interval. The duration cap happened to catch 1,142 of 1,842 such
  games; the remaining 700 were trusted with a start time known to be wrong. Now
  quarantined on the flag instead of on luck. `Immediate` and `Moves` both start when
  the row is created and are unaffected, which is why this costs no recall today: all
  5 tournament games among the 21 survivors are `Immediate`
- **a game whose `hashes` and `history` disagree in length is dropped.** `games.hashes`
  is `Array<Nullable<Int8>>` and `Game::hashes()` *filter_maps the NULLs away*
  (`db/src/models/game.rs:302`), compacting the array. Ply is the list index, and ply
  is what `MIN_PLY`, the run alignment and the branching bucket all key on, so one
  dropped element misaligns a whole game against every other. The two lists agree on
  80,323 of 80,323 real rows, so the check costs nothing — it exists so that a future
  backfill fails closed instead of silently shifting everybody's plies

Two properties are enforced rather than encouraged: an account appearing only as
somebody's *opponent* never gets a review file, and a pair where no single account
sits in both games implicates **nobody** — in that shape the review file withholds
*both* real-game usernames and prints only the game id, because position data cannot
say which of the two benefited and the other one is somebody's victim.

## What this does not do

- **No off-site engines — and this was measured, not assumed.** There is one cheap
  test available without an engine search per position: build a book of the moves the
  site's own bots actually played, keyed by the position they moved from (636,817
  positions from 765,396 bot moves — a hash lookup, no search), then score each
  account on how often it plays the book's move. It covers 13.4% of human-vs-human
  moves, and 299 accounts have ≥100 covered moves.

  **It does not separate.** Over human-vs-human moves only, and controlling for the
  rating confound (median match rate rises from 5.4% in the 1000–1199 band to 9.5% in
  1800–1999):

  | account | match rate | covered moves | rating |
  |---|---|---|---|
  | `deleted:e4e90728` | 27.9% | 2,386 | 1868 |
  | H1 *(live, ordinary account)* | 21.2% | 2,406 | 1864 |
  | H2 *(live, ordinary account)* | 19.2% | 3,675 | 1851 |

  The top scorer is the account the mirror detector already flagged, which is
  encouraging — but H1 and H2 sit just behind it at the same rating with
  *larger* samples, and nothing distinguishes "strong player" from "engine user" here.
  The ceiling is low too: the bots themselves only match each other at 27–38%, so the
  book's top move is "what the site's bots usually play", not "the engine move". A
  threshold that flagged 27.9% would flag ordinary strong players, and 27.9% is
  nowhere near playing engine moves every move.

  It is therefore **not implemented as a detector**. It would also only ever catch
  someone using *the site's own bots*; anyone running a different engine locally
  remains invisible. Recorded here so nobody has to re-derive it.
- **No causality — and it was measured, not assumed.** `move_times` is not a set of
  timestamps: `get_move_times` (`db/src/models/game.rs:725`) pushes the mover's
  *remaining clock* in nanoseconds. Differencing consecutive same-colour entries does
  recover time spent, and anchoring at `created_at` reconstructs a wall-clock time per
  move — that works, with a median relative error of 1.4–2.4% for Bullet, Blitz, Rapid
  and Classic.

  It is useless here anyway. In correspondence the per-move allowance is reset on every
  move, so `move_times` is a constant array: 13,696 of 16,141 correspondence games.
  Every bot game among the survivors is one of them (all show a constant
  `604800000000000` ns = 7 days). **0 of the 21 pairs have usable `move_times` on both
  sides**, so a per-move causality test would run on nothing. Building it would add a
  top tier unreachable by the attack this tool actually finds, so it is not built and
  **no tier above `p2` exists**.

  Re-measured independently on the current archive: 7 of the 52 games in flagged pairs
  have an informative `move_times`, and **0 pairs have it on both sides** — you need both
  to compare. The hypothesis that the array interleaves the two players' remaining clock
  is confirmed: on all 7, both parities are monotonically decreasing. So the field *is* a
  usable per-move clock where the time control does not reset; correspondence bot games
  are exactly where it does.

  **The fix is smaller than "add per-ply timestamps".** `game_hashes` already stores one
  row per `(game_id, turn)` and already has a `played_at` column — the right shape
  exactly. It is simply populated with the wrong value: `GameFinishContext::from_finished_game`
  sets `played_at: game.updated_at`, and `GameHash::from_engine_hashes` copies that one
  timestamp onto every row, so all rows in a game share the game's finish time. Writing
  the actual time each move was made would give per-move causality for every game
  regardless of time control, and would settle the direction question this tool cannot
  answer. Note that deriving it at game-finish from `move_times` would NOT work for the
  cases that matter, since those are the games where the array is constant.
- **No live detection.** This reads *finished* games, so it finds things after the
  fact — never during a tournament.
- **No identity.** `User::soft_delete` overwrites `normalized_username`, so a deleted
  account's owner is not recoverable from public data.
- **No distinction between relaying and analysis.** See "Evidence grades".

## Scoring

Matched positions inside one game are **not independent observations**. They are
consecutive states of one trajectory: a game containing the ply-45 position
necessarily contained the ply-44 one. Summing `log2(N/df)` over a contiguous run
therefore counts one coincidence once per ply — a 111-position mirror scored 1689
bits where the conjunction is worth about 15, and the error grew with run length,
i.e. it was largest exactly where the tool was most confident.

What is scored now: each maximal **run** contributes the surprisal of arriving at
its entry position, plus one branching term per step taken inside it. Runs at
different points in the game are genuinely independent and are summed.

The branching factor is measured rather than assumed — but only from positions the
corpus can actually observe branching (`df >= BRANCH_MIN_DF`). That estimate is
censored downward, so the score *understates* the evidence. For a tool that
generates accusations, understating is the correct direction.

**On the real archive that measurement is close to vacuous, and the README used to
oversell it.** Only 93 of 2,784,054 distinct positions (0.0033%) reach `df >= 20`,
because 99.5% of positions past ply 12 occur in exactly one game. The measured
factor comes out at 2.00 — numerically identical to `BRANCH_FALLBACK`. So in
practice each additional step inside a run is worth one bit, and the phrase
"measured, not assumed" describes the mechanism rather than the outcome. It is
still the right mechanism: on a denser corpus it would start to bind, and it errs
downward in the meantime.

### Known defect in the scoring model — unfixed, deliberately

Every run is charged a full `log2(N/df)` entry term, which treats each run as an
independent meeting of the two games. Measured on the live corpus (N = 75,934,
df = 2, b = 2.0, floor 20.00 bits):

| | runs | score | verdict |
|---|---|---|---|
| 3 matched positions, contiguous | 1 | 15.21 + 2×1.00 = **17.21** | dropped |
| the same 3 positions, scattered | 3 | 3 × 15.21 = **45.64** | admitted |

and, worse because it is simply incoherent, **the score is not monotone in the
evidence**: deleting six consecutive matched positions splits one run in two and
*raises* the total, so a corpus defect that punches a hole in a hash array increases
the reported evidence for the pair it damaged.

It is left unfixed because every constant-sized repair fails. A re-entry cost must be
> 0, or twenty scattered coincidences score the same as one and the F9 property
inverts; it must also be ≤ the per-step branching term, or splitting a run is still
rewarded — and that term is a measured median floored at 1.0, so it can legitimately
be 0 bits. No constant satisfies both. The repair is a *different evidence model*, and
picking one with no ground truth, during a validation pass, on a tool that names real
people, is worse than a documented defect.

What bounds the damage, measured rather than assumed, and re-measured on the current
corpus at the shipped default: **20 of the 21** survivors are a single run, so the
per-run entry term is charged exactly once and there is nothing for the non-monotonicity
to act on. The one exception holds 2 runs, 18 positions and 46 bits against the 23.3-bit
floor — comfortably clear either way. The defect is latent on this corpus, not active.

(A previous version of this paragraph said "27 of the 28 real survivors" and described
the exception as holding "31 bits as one run", which was both stale and
self-contradictory — the exception is the one that is *not* a single run. A validation
number quoted from memory is the same failure as the hardcoded `854` in `write_case`.
Recompute it — as this file now does at each release.)
`test_the_scoring_model_is_documented_as_non_monotone` pins the current wrong
behaviour so it cannot be quietly rediscovered; invert it when the model is replaced.

The floor is corrected for multiplicity against the number of pairs the scan
actually compared, which it already knows. An absolute bit total ignores that a scan
of 76k games examines billions of candidate pairings; a fixed count of "equivalent
near-unique positions" silently means something different at every corpus size, and
was scale-free only for `df=2` — which is exactly the case its regression test used.

## The negative control

There is no ground truth here: not one confirmed case has been used to calibrate
anything. `--null-permutations K` is the substitute, and it is the single most
useful number this tool produces.

It re-runs the scan `K` times with game start times reshuffled within each speed
class, preserving every duration and leaving every position, hash and player
relationship untouched. That destroys real concurrency and nothing else, so the
surviving pair count under permutation is exactly *how many pairs this detector
produces from coincidence alone*. It reuses the position index, so `K` permutations
cost `K` candidate loops and one index build.

- shuffled mean close to the observed count → the concurrency test is not
  discriminating and the survivors are position coincidences
- shuffled mean near zero → the observed count means something, and you can finally
  quote a number to a moderator

The result is printed at the top of every review file when the flag is used. **Run it
before acting on anything.**

### What it returned on the real archive

80,323 scraped rows → 75,934 loaded games, **K = 500 permutations**:

At the shipped default (`--alpha 1e-3`):

| | observed | shuffled (mean ± sd) |
|---|---|---|
| surviving pairs | 21 | 0.24 ± 0.48 |
| self-attributable | 6 | 0.04 ± 0.20 |

No shuffle out of 500 reached the observed count. 108 of 500 shuffles produced at least
one pair; the most any single shuffle produced was 2.

**What this control does and does not show.** It measures the *random-concurrency*
component only: the permutation destroys real concurrency and nothing else, so it
answers "would the live-window test admit these games by chance timing?" — and the
answer is a strong no (0.24 expected vs 21 observed). It does **not** measure precision:
it cannot model the *other* benign ways two games could genuinely be concurrent and
share a deep line (coordinated analysis, shared contemporary theory, activity bursts).
Real precision is unmeasured and stays that way until human-reviewed labels accumulate;
do not read "0.24 vs 21" as "≈99% precision" — an earlier draft made exactly that
overclaim and it was wrong.

A structural fact reinforces the timing result: every one of the 119 pairs the
permutation ever produced was already in the observed set. The shuffle never
manufactures a *new* pair; it only re-admits one of the same 21 when reshuffled
timestamps happen to land concurrently.

That is the right question, and it is only half of the problem. The other half —
*is sharing a run this deep itself surprising?* — is what `--epoch-control` answers,
and the two must be read together. Innocent pairs at least a year apart (which cannot
be relays) top out around **20 bits**. The shipped 1e-3 floor is **23.3 bits**, above
that ceiling, so those benign pairs no longer clear it at all; the flagged pairs run
28 to 125 bits (median 44). This is exactly why 1e-3 is the default — see the ALPHA
section: at the old 1e-2 floor of 20.0 bits the benign ceiling and the floor were level.

Neither control can distinguish relaying from using a bot as a live analysis board.
Nothing in the archive can. See "Evidence grades".

## Self-mirror — a rating correction, NOT a cheating finding

One account opens **two concurrent games against a bot, sitting White in one and Black
in the other**, and feeds each game's replies into the other. The bot then plays
itself. The account contributes no moves of its own and banks a result against an
engine far above its rating.

Found by investigating a single account on request. Two games created 53 seconds
apart, the same account against the same bot with the colours swapped, both 83 plies,
65 shared positions. Rating change **roughly +290 and +640** — about 900 points from
two *drawn* games, because Glicko-2 pays enormously for holding a 2550 engine at a fresh
RD. (Figures rounded: the exact two-decimal changes are a unique join key back to the
account, and account names are deliberately not in this public file. They are
in the scan output under `logs/`, which is gitignored for exactly that reason.)

Both existing detectors were blind to it, and one blind spot was self-inflicted:

| | why it missed |
|---|---|
| `find_pairs()` | both games hold a bot → `classify()` → `(None, None)` → "roles indeterminate" |
| `find_linked_bot_pairs()` | requires the two humans to **differ** — a rule I added, justified as "practising the same line twice links nobody" |

**The colour swap is the entire signature**, and it needs no result-based rule. Two
*same-colour* games against one bot sharing a line is just a repeated opening —
ordinary practice, and there are 20 such pairs in the corpus. For a colour-**swapped**
pair to share a line, the human's moves in one game must equal the bot's moves in the
other. Independent play cannot produce that.

**A draw by repetition is not the trigger and must never become one.** It is a
legitimate way to hold a stronger opponent. Nothing in the detector reads
`conclusion`, and on the real archive at most **one** of each pair's 13–88 shared
positions falls on a repeated ply — these are full-game mirrors, not repetition
artifacts.

**This is classified as a rating adjustment, not cheating.** There is no opponent: one
person, two games, a bot playing itself. What it distorts is a rating, so what it needs
is a rating correction. It is reported in its own section and never enters the review
queue or the registry — a queue headed by banned accounts is the wrong place for a
categorically different thing.

On the real archive: **17 pairs across 7 accounts.** Null 0.105 ± 0.338 over K=200,
no shuffle reached 17 (p ≤ 0.005), epoch control zero.

| account | pairs | net rating | deepest |
|---|---|---|---|
| S1 | 2 | **+911** | 65 |
| S2 | 7 | +199 | 45 |
| S3 | 1 | +181 | 23 |
| S4 | 1 | +48 | 38 |
| S5 | 1 | +18 | 15 |
| S6 | 1 | +0 | 37 |
| S7 | 4 | **−121** | 88 |

(S1–S7 are stand-ins; the scan prints the real usernames.)

The rating column is for the correction and is **not** a filter. Gating on profit
would have missed `S7`, which ran the technique four times and came out 121 points
down — and would be reasoning from outcome instead of mechanism.

## Linked accounts — the shape the relay pipeline cannot express

`classify` returns `(None, None)` when **both** games contain a bot seat, and the pair
is dropped as "roles indeterminate". The exemption is there for a real reason: a bot
plays the same replies to the same moves, so two people who each practise against
nokamute get similar games for free.

Measured, that reason only covers *shallow* agreement. Across the 838 pairs of games
whose only common account is the same bot and whose humans differ:

| shared positions | pairs |
|---|---|
| median | **1** |
| p90 | 4 |
| ≥ 25 | 11 |
| ≥ 50 | 2 |

The pairs actually being dropped on the live archive shared **84, 73, 46, 40, 33 and
25**. Bot determinism does not produce that; it produces one.

`find_linked_bot_pairs()` detects it: two concurrent games, each one human against one
bot, different humans, reproducing a trajectory between them. Same eligibility, same
repertoire exemption, same multiplicity-corrected floor — only the role logic differs.

**It implicates nobody of beating a human.** Neither game has a human opponent, which
is exactly why the relay pipeline cannot see it. What it establishes is that two
accounts are **linked**, which is the fact a moderator needs and cannot otherwise get
from position data. The registry signal says so in words.

On the real archive: **10 pairs, 2 account pairs.**

| linked accounts | pairs | deepest |
|---|---|---|
| `deleted:1f34287c` ↔ `deleted:e4e90728` | 7 | 84 positions, 98 bits |
| L1 ↔ L2 *(two live accounts — names in the scan output, not in this public file)* | 3 | 47 positions, 61 bits |

## `book_follow.py` — opening-explorer use past the opening

The one attack with **no trace in the archive at all**. The explorer is queried by
position and ranks continuations by how often they were played; anyone can paste a
position out of their own live game and play whatever comes top. No engine, no second
account, no bot — and the lookup never becomes a game, so nothing else in this directory
can see it. Everything else here works by pairing two games; here there is only one.

```bash
py -3 scripts/fair-play/book_follow.py --archive archive.jsonl --top 25
```

What is measurable is the consequence. Three decisions carry the whole thing:

1. **The book excludes each account's own games.** Without that, an account with a pet
   line "follows the book" that its own earlier games created, and the heaviest
   repertoire players — the most ordinary thing in the game — come out top. Same trap
   `_repertoire` exists for in `mirror_scan.py`, and it would have inverted the result.
   `book_move(slot, exclude_uid)` subtracts the player's own contribution from
   precomputed counts and re-takes the argmax; the naive rescan went quadratic on popular
   positions and did not finish in ten minutes.
2. **Only positions where a book actually exists count.** A continuation needs
   `MIN_BOOK_PLAYERS = 3` other players behind it. Past ply 12, 96% of archive positions
   were reached by exactly one game, so the denominator is small *by nature*.
3. **The baseline is a rating window centred on the account**, widened 200 → 400 → 600
   until it holds 8 peers, and an account with no peers even at ±300 is reported as
   **unscoreable** and cannot be flagged. It used to fall back to the whole-site median,
   which for anyone at either tail measures "is this player unlike the average player" —
   true by construction, and nothing to do with the explorer. The windows are centred
   rather than snapped to a multiple because an aligned band grows in one direction, so a
   sparse account near an edge never reaches peers a few points across it.

**The column that matters is deep-vs-shallow.** Somebody reading the explorer mid-game
should be unusually booked *past* the opening relative to how booked they are *inside* it.
Somebody who is simply well prepared is booked in both.

On the real archive: 77,423 games with usable ply indexing, 20,340 positions with enough
traffic to carry a book, 115 accounts with ≥ 40 judgeable moves past ply 8. The
multiplicity-corrected cut is z ≥ 3.00 and **one** account clears it — at 49.1% deep
against 43.8% shallow, which is preparation, not lookup. Neither account just below the
cut fits the mid-game-lookup shape either: the next (z ≈ 2.8) is actually more booked
*deep* than shallow by a couple of points, and the one after (z ≈ 2.7) is more booked
*shallow* than deep — i.e. scattered on both sides of the diagonal, not stacked above it.
**Nothing here reaches the registry**, by design.

> The single worst bug found in this whole pass lived here. `hashes[p]` is the position
> that `history[p+1]` was played from, and White moves at **even** history indices
> (verified on the archive: index%2==0 carries a `w` piece 23,888 times against 12). It
> was written `w if p % 2 == 0 else b` — the opposite — so every judged move was credited
> to the player who did not make it. Nothing crashed; the output was a complete,
> well-formatted, plausible table about the wrong people, and it had already been
> reported. One named function, `mover_of(p)`, now decides it in one place.

## `engine_check.py` — the second stage, and why it is not a screen

The shape `find_pairs` structurally cannot see is an engine running off-site: there is no
second game to match positions against. This scores an account against the production
transformer's **policy**, conditioned on how much choice the position actually offered.

```bash
# 1. the net's eval server (READ THE GPU WARNING IN THE MODULE DOCSTRING)
py -3 tools/az_eval_server.py data/nets/<net>.pt data/az/_serve --serve-port 8899
# 2. fit the reference from the archive's own bot flag
py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --calibrate --games 120
# 3. score one account the position evidence already selected
py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --uid <uid>
```

One `analyze sims 1` is **one forward pass** — the policy head ranks every legal move in
that pass, so there is no search and a 60-ply game costs about 4 ms. The cost objection to
engine-based checking does not survive contact with an AZ-style policy head. Calibration
needs no confirmed cheaters because the archive labels a *different* question: the `bot`
flag marks engine play. Policy entropy over the legal moves is the difficulty
conditioner — agreeing with the net where it saw one obvious move proves nothing, so the
**hard** column is the whole signal.

### The verdict, measured: it cannot stand alone

| population | hard plies | top-1 \| hard |
|---|---|---|
| engine (bot seats) | 7,086 | **24.7%** |
| human vs human | 35,265 | **19.3%** |

That pooled gap looks usable and **is not**. Per *account*, over the 33 human accounts
with ≥ 150 hard plies: median 21.2%, sd 4.8, range 12.0–29.1%. The engine sits **0.7
account-sd** above the human median and **6 of 33 ordinary accounts already match or
exceed it**. A threshold at the engine's rate names those six. Pooling hides the
between-account spread; two populations can differ in the mean while their members overlap
almost completely, and that is exactly the shape here — the same wall the archive-only
difficulty experiment hit.

So this stays a **second stage**, run on an account the position evidence already
selected, where the prior is "this specific account, for this specific reason" rather than
"one of 5,198". It is also the piece that improves for free: a stronger net widens the
gap, and `registry.py export-labels` accumulates the supervised labels a real classifier
would need.

### Two defects that made every earlier number wrong

**Move-notation aliasing.** A Hive move names its destination *relative to a neighbour*,
so a cell with several occupied neighbours has several equally valid spellings — `wG1
wA1/` and `wG1 \wS1` are the same move whenever wA1 sits west of wS1. The played move was
compared as a **string** against the net's candidate list, and the candidates come out of
the engine's own generator. Measured over 3,209 real plies: only **71.4%** of archive move
strings appear verbatim in the engine's `validmoves`. The other 28.6% scored `rank = -1` —
"did not play the net's move" — regardless of what the net thought of them, and the
aliasing rate rises with how crowded the board is, so it biased late play hardest.

Reading the engine's echo back does not fix it: `play` echoes the GameString with the move
**verbatim** (measured: 0 of 1,707 re-rendered), so the engine never states a canonical
form. `resolve()` maps both sides to absolute axial coordinates instead, at no extra
engine calls, and it is validated against the engine's own move generator: over those
3,209 plies the archive move resolves to **exactly one** entry of `validmoves` 3,187
times, to none 0 times, to more than one 0 times; the remaining 22 are passes. On identical
plies the fix moves the hard column from 15.5% → 24.7% (engine) and 11.6% → 19.3% (human).

**A rejected move used to be swallowed.** The engine keeps its old board after `err`, so
every later ply was scored against a position the player never faced — and with the side
to move flipped if the rejection was a missing pass. Nothing crashed; the rows quietly
described a different game. `IllegalReplay` now abandons the game, and the run reports how
many it abandoned rather than presenting a confident number over what was left.

The first pair is independent corroboration of the linkage seat correspondence already
inferred. **The second was invisible to the relay pipeline entirely.**

Both controls pass, and more cleanly than the main detector: permutation null
**0.035 ± 0.184** over K=200, no shuffle produced more than 1, none reached 10
(p ≤ 0.005); the epoch control finds **zero** pairs a year or more apart.

## Seat correspondence — which seat received the moves

`seat_correspondence(pair)` resolves *which seat* of the rated game the oracle's bot
corresponds to, without needing per-move timestamps.

`engine/src/hasher.rs:27` XORs a side-to-move term into the canonical hash, so two
games sharing a position are at the same parity and the colour mapping between them is
the identity. Measured on the current archive: **all 21** surviving pairs have a ply
offset set of exactly `{0}` — constant within every pair, and zero in every pair. The
report no longer states this figure from memory; `write_case` counts it per run, and says
so explicitly when a file carries no independently-known subject to check against. In a
relay the person sits in the
oracle game playing their *opponent's* colour — the bot has to produce moves for their
own — so the bot's colour is the colour that received the moves.

It resolves a seat for **all 15** unattributable pairs. On the 6 self-attributable
pairs, where the subject is known independently, it reproduces the right answer
**6 times out of 6, with 0 disagreements**.

There is one further check, and it is the strongest evidence in this directory that the
rule points the right way. In every one of the unattributable pairs (15 at the shipped
default), one rated-game seat is an account that has since been **removed** from the site
and the other is a live named player. A coin flip would name the removed account about
half the time. The rule named the removed account **every time, and a live community
member zero times.** The removals predate this tooling and no threshold was touched to
produce that result — but it is *one operation* seen many times, not that many
independent confirmations, and it must not be quoted as if it were.

### When it refuses to answer

`mark_seat_conflicts` withdraws the seat claim for any rated game whose seat resolves
**two ways**. Two colour-swapped oracles mirroring one rated game resolve *opposite*
seats — which is what one person opening two analysis boards produces — and before the
guard both rated-game players got a review file naming the other as excluded. That is the
named-victim failure, from a single position component, with the victim in bold. At most
one answer can be right and the position data does not say which, so both are withdrawn
and the report returns to withholding both names.

It must run over the **full** survivor set, not per pair: a contradiction is only visible
when two pairs about one rated game are compared to each other. On the current archive
**0 of 21** pairs are contradicted, so the guard costs nothing today.

### What it changed on the real archive

Before, the 15 alt-shape pairs looked like two unrelated accounts and 30
interchangeable strangers. After:

| ran the oracle | → seat that received the moves | pairs |
|---|---|---|
| `deleted:96d76861` | `deleted:e4e90728` | 9 |
| `deleted:1f34287c` | `deleted:e4e90728` | 6 |
| `deleted:e4e90728` | *(same account in both games)* | 5 |

Two buffer accounts feeding one main account, which also ran five mirrors under its
own name — **20 of the 21 surviving pairs are one operation.** The main account's
review file went from 5 pairs to 20 and now heads the queue. Fourteen opponents are
positively **cleared**.

### What it still does not establish

It names an **account**, not a person. That account may itself be an alt, and linking two
accounts to one human is not something the archive can do.

Earlier versions of this file, and of `write_case`, said that step "needs the database —
shared address, session or payment data". **That was wrong, and it was never checked
against the schema.** What `db/src/schema.rs` actually offers:

| field | what it can show | use |
|---|---|---|
| `push_devices.device_token` | the same physical device on two accounts | **direct linkage, the strongest thing here** |
| `users.email` | same or related address | useful |
| `users.created_at` | accounts created minutes apart | weak |
| `push_devices.platform`, `locale`, `last_seen_at` | same device family and locale, active together | weak |
| `email_request_log.ip` | the *only* `ip` column in the schema | **effectively unusable — see below** |
| sessions, login history, payment data | — | **do not exist** |

Two limits that decide this:

* `push_devices` only has rows for accounts that enabled push notifications, so it is
  absent far more often than present, and there is no fallback behind it.
* `email_request_log.ip` looks like the answer and is not. It is a rate-limit log written
  only by `forgot_password.rs` (password-reset requests, nothing else — not logins, not
  gameplay), and `apis/src/jobs/email_cleanup.rs` calls
  `EmailRequestLog::delete_before(now - 1 day)`. **The rows are purged after 24 hours**,
  so by the time a scan surfaces a pair they are long gone.

Do not add IP logging *because of this tool*. If it is worth having, it is worth deciding
on its own terms, with a retention period chosen deliberately — a fair-play detector is a
bad reason to start retaining address data.

This finding is also conditional on the pair being a relay at all, which no archive field
establishes. **The grade stays `p3` for exactly that reason**, and no tier above `p2` was
added. Where the linkage cannot be made, the correct outcome is that the account stays
`suspicious` — not that the bar moves.

The most reliable half of the output is the **exclusion**: it says who each pair is
*not* about. Before this, the report withheld both names, which protected the person
who benefited and the person played against equally.
`test_report_names_at_most_the_seat_that_received_the_moves` is the restated F10
guarantee — the seat the analysis clears must be *stated as cleared*, never presented
as the subject. Do not weaken it.

In the registry the seat-derived account carries a signal that says so in words
(`"...seat correspondence says this seat received the moves (inference — account
linkage needs the database)"`), so a moderator running `registry.py show` can tell it
apart from an account observed in both games.

## Recall is not measurable. The evasion surface is.

There is no confirmed case, so there is no denominator: nothing here can say what
fraction of cheating it catches, and a test suite that constructed cheaters until it
caught them would only be writing the cheater to match the detector.

What *can* be measured is what a person who has read this file can do about it. Every
threshold below is a published constant in a public repository. Each row injects one
relay into the real 75,934-game archive, changing one thing:

| evasion | caught? | what it costs the cheater |
|---|---|---|
| faithful relay, 6+ plies | **yes** (20.2 bits vs 20.0 floor) | — |
| the same relay in Blitz / Rapid | **yes**, `p1` | — |
| relay only the first 4 plies | no | almost nothing is gained |
| open the oracle 41% in (`MAX_START_PCT`=40) | no | no advice for the first 41% of the game |
| ~~keep playing the bot game past the real game~~ | **now caught** | **closed — see below** |
| pre-play the whole line in 3 games (`REP_MIN_GAMES`) | no | must know the line in advance — see below |
| 7 accomplices on the line (`MAX_COMPONENT_GAMES`=6) | no | seven confederates |
| human confederate rated < subject+100 | no | confederate must be weak |
| confederate answering in a faster format | no | confederate must answer at blitz speed |
| both games in one tournament | no | both games must be in one tournament |
| real game unrated | no | no rating gained — pointless |
| oracle on a second account | detected `p3`, **and the seat now names an account** | one extra account |

**The cheapest evasion is closed.** `MIN_OVERLAP` normalised the overlap by the
*oracle's own* duration, so lengthening the oracle shrank the ratio and changed nothing
else — and since `end` is `last_interaction`, the last **move**, a cheater simply kept
playing the bot game after the real game finished. Measured: on a median 72.5h
correspondence game, 15.9h of extra play was still caught and 18.1h was invisible.
`MIN_COVERAGE` normalises by the *real* game's span instead, which is immune by
construction because overlap is capped by the real game. The evasion is caught at every
threshold from 0.00 upward — the repair was the change of statistic, not the constant.
See `MIN_COVERAGE` for the sweep and the cost (3 pairs, all `p3`; all 6
self-attributable pairs kept; null mean 0.212 → 0.160).

**The repertoire exemption is not the free pass it looks like.** `REP_MIN_GAMES = 3`
reads like a two-minute permanent immunity: play the line in three throwaway games and
it can never be evidence. It is not, because a relay cannot know the line in
advance — the bot chooses the replies. The exemption is per-position and degrades
gracefully: on a 60-ply relay, pre-playing the first 50 of 60 positions is still caught
at 24 bits, and it takes ~55 of 60 to go dark, i.e. knowing almost the whole game
before it starts.

**The alt-account evasion is no longer an attribution evasion.** It is still detected
only at `p3`, but seat correspondence now names the seat that received the moves, so
the account that gained the rating reaches the queue instead of only the buffer account
that ran the bot.

`EvasionTests` pins all of this, so changing a threshold reports what it did to the set
of people who can walk past.

## Calibration of the two free constants

`ALPHA` and `P1_MIN_RUN` were originally set by judgement. Both are now derived from
the two negative controls, on the 75,934-game archive. Neither was chosen by looking
at any account, and no threshold here was moved to change any particular verdict.

### `ALPHA` — the multiplicity-corrected evidence floor

Stated criterion: hold the *expected false-pair count across the whole scan* below 1.
Measured by sweeping alpha and re-running the permutation null at each value:

| alpha | floor | observed pairs | self-attr. | E[false pairs] | criterion |
|---|---|---|---|---|---|
| 1e-6 | 33.3 bits | 20 | 6 | ~0.1 | met |
| 1e-4 | 26.6 bits | 21 | 6 | ~0.1 | met |
| **1e-3 (shipped)** | **23.3 bits** | **21** | **6** | **0.05** | **met** |
| 1e-2 | 20.0 bits | 26 | 6 | 0.27 | met |
| 1e-1 | 16.7 bits | 27 | 6 | ~0.3 | met |
| 1.0 | 13.4 bits | 27 | 6 | ~0.3 | met |

*Observed-pairs and self-attr. columns recomputed on the current code. E[false pairs]
values are order-of-magnitude from per-alpha null sweeps; the shipped 1e-3 row is the
K=500 permutation mean at that setting. The point does not depend on the third decimal.*

**The multiplicity criterion does not bind** — it is satisfied six orders of magnitude
either side of the shipped value, because the bit floor is not what controls false
pairs on this corpus; the live-window test is.

The criterion that **does** bind, and that sets the shipped default, comes from the
epoch control: *the floor should exceed the highest score reached by a pair that cannot
possibly be a relay.* That maximum is 20 bits. At the old 1e-2 default the floor was
exactly 20.0 — **level with the benign ceiling, not above it** — and the weakest
survivors sat at 20–23 bits, inside the range innocent position-sharing demonstrably
reaches. The shipped default is now **1e-3**, whose 23.3-bit floor clears that ceiling:
the weakest survivor is 28.2 bits, and the five pairs that sat in the 20–23 danger band
(all p3, no self-attributable pair among them) are dropped. This is the precision-first
default the project's volunteer-moderation constraint calls for; `--alpha 0.01` remains
for wider research sweeps.

By that derivation `ALPHA = 1e-3` (floor 23.3 bits) is the defensible value. Its cost,
measured rather than estimated: **6 pairs, all of them `p3` alt-shape, and zero
self-attributable pairs.** Both `p1` pairs and all four `p2` pairs survive it.

The honest caveat: the epoch control found only **2** far-apart pairs, so "the innocent
maximum is 20 bits" rests on a sample of two and the tail is unmeasured. The data
cannot strongly separate 1e-3 from 1e-2. `ALPHA` is left at 1e-2 with this written
down; `--alpha 1e-3` is one flag away, and is the value to prefer if a moderator's
time is the scarce resource.

### `P1_MIN_RUN` — the top tier's run-length gate

**The permutation null cannot calibrate this constant at all,** which is worth stating
because it is not obvious. `chain` is a function of the position data alone, and
`permute_times` leaves positions untouched, so any pair appearing in both the observed
set and a shuffle has an identical `chain` — checked over 60 shuffles: 8 such pairs, 8
identical, 0 different. The null is blind to it.

The epoch control is the right instrument, because it measures what innocent shared
theory looks like. Both far-apart pairs have `chain = 6`. `P1_MIN_RUN = 40` is
therefore ~6.7× the deepest run any demonstrably-innocent pair reaches, and is kept.
Observed self-attributable bot-oracle pairs have chains of 111, 62, 47, 30, 22 and 11,
so 40 separates {111, 47} from the rest; lowering it to 30 would add one more. With
n = 2 innocent pairs the data cannot distinguish 30 from 40, and since the grade sets
*queue order only* and decides nothing, being conservative costs a `p2` sorted below a
`p1` rather than a missed case.

One thing that could have made `P1_MIN_RUN` mean less than it says: `RUN_GAP = 6`
allows a "run" of 40 matched positions to span up to 240 plies, which would not be
"one unbroken run". On the real survivors it does not — every `p1`/`p2` pair has
`chain` exactly equal to its ply span plus one, i.e. genuinely contiguous.

## Tests

```bash
py -3 -m unittest discover -s scripts/fair-play -p 'test_*.py' -v
```

134 tests. Two are named after bugs that would have produced accusation files about
the cheater's *victims* — `test_victim_is_never_the_subject` and
`test_unattributable_pair_implicates_nobody`. A third,
`test_contradictory_seat_resolutions_withhold_both_names`, covers the same failure
reached a different way. Please don't delete them.

`EngineNotationTests` drives `score_game` through a small fake engine object with three
methods, so the notation and abandon-on-error guarantees are covered **without a GPU** and
run in CI. `test_the_played_move_matches_the_nets_top_choice_despite_the_spelling` is the
one that pins the aliasing fix.

`ArchiveShapeTests` runs against `fixtures/archive_shape.jsonl`: 23 rows cut from the
real 80,323-row archive, one per interesting shape, with usernames, uids, game ids and
**position hashes** all replaced. The hashes are scrubbed too, not only the names —
the archive endpoint accepts a `position_hash` query, so a real hash is a lookup key
back to a real game and therefore back to real people.

That class exists because every field shape in `load()` was inferred from the Rust
source and had never been checked against real output — and because *every* mismatch
fails silently by disabling an exemption rather than by raising. `_tour_id` returning
`None` because no key matched is exactly how the tournament bug worked, and it would
have looked like a clean run. Measured against the real archive: `tournament_id` is
present and `_tour_id` fires on 100% of the 4,864 tournament games; uids are 32-char
hex (`uuid::Uuid` serialises as *bytes* under CBOR, since ciborium's
`is_human_readable()` is `false`); `game_id` is unique across all 80,323 rows;
timestamps parse on 100% of rows; `len(hashes) == len(history)` on 100% of rows; no
hash is ever 0 or negative; `last_interaction` is present on 99.99%.

`RegistryWiringTests` runs `main()` end-to-end over a corpus containing a mirror, a
cluster, an alt-account pair and victims, and asserts the registry ends up with
`suspicious` for exactly the two intended accounts, nothing at all for any victim,
unattributable real-game player, clustered player or bot, and nothing anywhere above
`suspicious`.

The `RegressionTests` class is a bug-per-test record of an adversarial review. Every
one of those shipped, and mutation testing had full coverage of the rules and caught
none of them, because each is either a boundary or an interaction between two rules
that are individually fine. Two of the old tests were *worse* than absent —
`test_floor_is_scale_free` and `test_evidence_is_not_duplicated_on_rescan` each
pinned the single input on which the property could not fail, so they read as
coverage while providing none. Both are rewritten to be falsifiable.

`test_unrelated_games_do_not_change_a_pairs_verdict` is the property the old suite
could not express: a verdict about two people must not depend on games involving
neither of them.

## What the real archive actually looks like

Measured over 80,323 scraped rows, because several design decisions turn on it:

- **100% of games are `game_type = MLP`.** The game-type filter, which exists because
  the canonical hash omits `game_type`, currently excludes nothing. Keep it — it is
  correct and it costs nothing — but do not count it as a working control.
- **99.5% of positions past ply 12 occur in exactly one game** (2,768,986 of
  2,784,054). Only 14,644 positions are *eligible* at `2 <= df <= 8`. Hive positions
  are essentially unique after the opening, which is why this approach works at all.
- **All 6 rows with a null `last_interaction` also have zero hashes**, so they are
  dropped for length before the `updated_at` fallback can run. On this corpus that
  fallback is unreachable and the fail-closed timing rule costs no recall from that
  branch — every untrusted game comes from the duration cap or the `game_start` flag.
- **All 21 surviving pairs are Correspondence vs Correspondence, both rated**, though
  Correspondence is only 18% of the corpus. Plausibly real — you cannot replay a
  bullet game into a bot — but it also means the live-window test has never been shown
  to detect anything in a timed format, and its recall there is unmeasured.
- **The corpus is a point-in-time snapshot.** `pull_archive.py` now pages by keyset:
  the first request of each `(result × speed)` stream is OFFSET page 1 (the newest 50,
  always shallow), and every request after it threads the server's returned
  `next_batch` token as `batch_token` (`db/src/helpers/games_query_builder.rs:52`
  switches to keyset when the token is present), so a game finishing mid-crawl cannot
  shift an offset boundary and drop a row. That makes a single crawl reproducible, but
  it is still a snapshot: games finishing after it are simply absent until the next
  crawl, and the scan says nothing about them.

## Status

Research tooling, not a deployed system. It has **never been validated against a
confirmed case**, so its false-positive rate is unmeasured except by the permutation
control above, and its recall is unknown. Treat every output as a question for a
human, not an answer.

The detector has now been run on the real archive and both negative controls pass
(see "The negative control"). That means the survivors are not timing coincidences.
It does **not** mean they are cheating: the tool still cannot separate relaying from
using a bot as an analysis board, and 15 of the 21 pairs are the unattributable shape
where position data alone cannot say which of two people benefited — seat correspondence
now names a seat there, but a seat is an account and an account is not a person.
