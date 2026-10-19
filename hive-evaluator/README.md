# hive-evaluator

Takes finished games from hivegame.com's eval queue, searches every position with the
StockBee engine and posts back which moves were inaccuracies (`?!`), mistakes (`?`) and
blunders (`??`), each with the move the engine would have played.

It pulls work over HTTP (`/api/v1/evals/…`), so it can run on the web server or on any
other machine, including a rented GPU box.

## How a game is searched

Every position is searched from scratch at 64 simulations (the StockBee author's suggestion). Each move that looks like it
lost two or more points of win chance has both of its positions searched again at 800.
On 13 corpus games this graded like a plain 800-sim search at about 80% of the cost; a
first pass below 50 sims missed about one real mistake in six. The measurements used a
50-sim screen; 64 was not measured separately and costs slightly more. Change the levels with
`--screen-sims`, `--threshold` and `--full-sims`.

A move that ends the game is scored by its result: won 100, drawn 50, lost 0. Whether the
game ended is the site's call (`hive_lib`), not StockBee's: StockBee counts some positions
as repeated that Hive's rules do not, such as one where a thrown piece is stunned. A move
that leaves the opponent nothing but a pass is scored from the position after that pass.

On the prod box (i7-7700, no GPU) the default layout, three eval servers with two torch
threads and one engine each, takes about a minute for a typical 41-move game.

## Running it

The site needs a shared secret for workers. Without it every worker request is refused:

```sh
# in the site's .env
EVAL_WORKER_TOKEN=<long random string>
```

The worker needs a built StockBee directory (`build/stockbee`, `build/libgraph_features.so`,
`tools/`, the net) and a Python with `torch` and `numpy`:

```sh
cd stockbee
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build -j2 --target stockbee
g++ -O2 -shared -fPIC -std=c++17 -fopenmp src/graph_features.cpp -o build/libgraph_features.so
python3 -m venv venv && venv/bin/pip install numpy torch --index-url https://download.pytorch.org/whl/cpu
```

With nix, `nix develop .#evaluator` has cmake and a Python with torch and numpy instead; the
default shell leaves them out, as torch is a large download.

On prod, `scripts/run-evaluator.sh` does the rest (see `scripts/README.md`, Evals). By hand,
at the lowest priority so the site always comes first:

```sh
EVAL_WORKER_TOKEN=<same secret> \
STOCKBEE_DIR=/path/to/stockbee STOCKBEE_PYTHON=/path/to/stockbee/venv/bin/python \
nice -n 19 hive-evaluator --base-url http://localhost:3999 --worker prod-1
```

`--worker` must be unique per running worker. The eval servers listen on 127.0.0.1 from
port 41871 up, one port per server; move them with `--base-port`. `RUST_LOG=debug` also
prints the eval servers' own logs.

## When things go wrong

- A worker that stops reporting for five minutes loses its eval; the site queues it
  again, and gives up after three attempts. Should the first worker deliver after all, its
  result still counts and the second worker stops.
- Every request to the site gives up after 30 seconds, so a stalled connection cannot hang
  the worker.
- If the site cannot be reached when a finished eval is sent back, during a deploy say, the
  worker keeps the eval alive and tries again for about four minutes.
- If an engine or eval server dies, the worker hands the eval back, which queues it again
  within the same three attempts, and restarts all of its processes.
- A failed eval shows as failed in the Evals tab, where any logged-in user can request it
  again.
