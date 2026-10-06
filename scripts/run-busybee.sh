#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="$HOME/.config/busybee/env"
cd "$(dirname "$(readlink -f "$0")")/../busybee"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Missing env file: $ENV_FILE" >&2
    exit 1
fi
if [[ ! -f venv/bin/activate ]]; then
    echo "Missing virtualenv: $PWD/venv (see busybee/README.md)" >&2
    exit 1
fi

set -a
. "$ENV_FILE"
set +a
. venv/bin/activate

BUSYBEE_HOST=127.0.0.1 exec bash launch.sh
