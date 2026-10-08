#!/usr/bin/env bash
set -euo pipefail

# cron and non-login tmux windows do not put rustup's cargo on PATH.
export PATH="$HOME/.cargo/bin:$PATH"
ENV_FILE="$HOME/.config/hive-evaluator/env"
cd "$(dirname "$(readlink -f "$0")")/.."

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Missing env file: $ENV_FILE" >&2
    exit 1
fi

set -a
. "$ENV_FILE"
set +a

# Through nginx's :3999, so the worker always reaches the live slot, which also keeps the
# site's "is a worker running" note on the instance users talk to.
export HIVE_URL="${HIVE_URL:-http://localhost:3999}"

# The site and deploys come first: the build stays below deploy.sh's nice 10, and the worker,
# with the engine and eval servers it starts, runs at the bottom.
nice -n 15 cargo build --release -p hive-evaluator
exec nice -n 19 "${CARGO_TARGET_DIR:-.cargo/target}/release/hive-evaluator" \
    --worker "${EVAL_WORKER_NAME:-prod-1}"
