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
  again, and gives up after three attempts.
- A failed eval shows as failed in the Evals tab, where any logged-in user can request it
  again.
- If an engine or eval server dies, the worker reports the eval as failed and restarts
  all of its processes.
