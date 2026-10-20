# hive-evaluator

Takes finished games from hivegame.com's eval queue, searches every position with an engine
and posts back which moves were inaccuracies (`?!`), mistakes (`?`) and blunders (`??`), each
with the move the engine would have played.

It pulls work over HTTP (`/api/v1/evals/…`), so it can run on the web server or on any
other machine, including a rented GPU box. Any engine that speaks the protocol below can do the
searching; which one, and how it is started, is set in an engines file.

## How a game is searched

Every position is searched first with the engine's `screen` budget. Each move that looks like
it lost `threshold` (default 2) or more points of win chance has both of its positions searched
again with the `full` budget. On 13 corpus games, a 50-sim screen, a threshold of 2 and
800-sim searches graded like a plain 800-sim search at about 80% of the cost; a screen below 50
sims missed about one real mistake in six.

The graph and the marks must agree: a move that swings by an inaccuracy's worth (5 points)
or more is marked for it. A swing the move cannot be marked for, because the engine chose the
move itself or because it seems to gain, means the screen misjudged a position, so both of the
move's positions are searched again with the full budget too. A position the full search
corrects is checked against the move before it as well, until nothing new turns up, and every
position counts at its deepest search, so a move starts where the previous one left the game.

In one local game the screen gave a winning side 1% and hid three Black blunders; with these
rules they are marked, and no swing is left unmarked. It searched 37 of the game's 46
positions in full instead of 26, about 90% of the cost of a full search of every position
instead of 65%, so the measurements above (taken before these rules) overstate what the screen
saves.

A move that ends the game is scored by its result: won 100, drawn 50, lost 0. Whether the
game ended is the site's call (`hive_lib`), not the engine's. A move that leaves the opponent
nothing but a pass is scored from the position after that pass.

## Engines

The engines file (default `~/.config/hive-evaluator/engines.toml`, or `--engines-file` /
`EVAL_ENGINES_FILE`) names one or more engines. The worker runs one of them: the only one, the
one named by `use`, or the one named by `--engine` / `EVAL_ENGINE`.

```toml
use = "deep"

[engines.deep]
command = "/opt/deep/bin/engine"          # started once per engine; paths are best absolute
args = ["--server-port={port}"]           # {port}: the port of its helper, if it has one
dir = "/opt/deep"                         # working directory (optional)
env = { DEEP_THREADS = "1" }              # extra environment (optional)
analyze = "analyze nodes {budget}"        # the analysis command; {budget} is filled in
count = 1                                 # engines, or engines per helper (default 1)
fingerprint = ["/opt/deep/weights.bin"]   # files that tell its versions apart (optional)
screen = 100                              # budget for every position
full = 1000                               # budget for the rechecks
threshold = 2.0                           # optional

# Optional: a process the engines talk to, such as a server holding a net.
[engines.deep.helper]
command = "/opt/deep/bin/server"
args = ["--port", "{port}"]
dir = "/opt/deep"
env = {}
ready = "listening"                       # text in a line of its output once it is up
ready_timeout_secs = 900                  # optional
count = 2                                 # helpers, on ports first_port, first_port + 1, …
first_port = 40100

[engines.quick]
command = "/opt/quick/engine"
analyze = "analyze depth {budget}"
count = 4
screen = 4
full = 8
```

Each eval records the engine's `id`, a digest of the fingerprint files and the budgets, so
evals from different engines or versions can be told apart.

### Protocol

The engine speaks [UHP](https://github.com/jonthysell/Mzinga/wiki/UniversalHiveProtocol) on
stdin and stdout. Every reply ends with a line `ok`; a line starting with `err` is a refusal.

- On start it prints its `id …` line, as for `info`.
- `newgame <GameString>` sets up the position to search, e.g.
  `newgame Base+MLP;InProgress;Black[2];wL;bL wL-;wQ /wL`.
- `validmoves`: an answer of just `pass` means there is nothing to search.
- The `analyze` command answers with a line
  `cand <best move> | winprob <0–100> | pv <move>;<move>;…`: the move it would play, the side
  to move's win chance, and the line it expects. `pv` is optional; other fields and lines are
  ignored.

The engine may spell a move differently from the game record; the evaluator recognises two
spellings of one move itself.

## Running it

The site needs a shared secret for workers. Without it every worker request is refused:

```sh
# in the site's .env
EVAL_WORKER_TOKEN=<long random string>
```

On prod, `scripts/setup-evaluator.sh` sets up the worker and `scripts/run-evaluator.sh` runs it
(see `scripts/README.md`, Evals). By hand, at the lowest priority so the site always comes
first:

```sh
EVAL_WORKER_TOKEN=<same secret> \
nice -n 19 hive-evaluator --base-url http://localhost:3999 --worker prod-1
```

`--worker` must be unique per running worker. `RUST_LOG=debug` also prints the helpers' logs.

## When things go wrong

- A worker that stops reporting for five minutes loses its eval; the site queues it
  again, and gives up after three attempts. Should the first worker deliver after all, its
  result still counts and the second worker stops.
- Every request to the site gives up after 30 seconds, so a stalled connection cannot hang
  the worker.
- If the site cannot be reached when a finished eval is sent back, during a deploy say, the
  worker keeps the eval alive and tries again for about four minutes.
- If an engine or helper dies, the worker hands the eval back, which queues it again within
  the same three attempts, and restarts all of its processes.
- A failed eval shows as failed in the Evals tab, where any logged-in user can request it
  again.
