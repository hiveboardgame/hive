#!/usr/bin/env bash
set -euo pipefail

# cron and non-login tmux windows do not put rustup's cargo on PATH.
export PATH="$HOME/.cargo/bin:$PATH"
ENV_FILE="$HOME/.config/hive-hydra/env"
cd "$(dirname "$(readlink -f "$0")")/../hive-hydra"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Missing env file: $ENV_FILE" >&2
    exit 1
fi

set -a
. "$ENV_FILE"
set +a

exec cargo run --release -p hive-hydra -- -c ./hive-hydra.yaml
