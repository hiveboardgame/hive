#!/usr/bin/env bash
set -euo pipefail

SESSION=hive
ROOT=/home/drone/hive

if ! tmux has-session -t "$SESSION" 2>/dev/null; then
    tmux new-session -d -s "$SESSION" -n hive -c "$ROOT" \
        "journalctl -f -u hive@blue -u hive@green; exec \$SHELL"
    tmux new-window -t "$SESSION" -n hydra -c "$ROOT" \
        "$ROOT/scripts/run-hydra.sh; exec \$SHELL"
    tmux new-window -t "$SESSION" -n busybee -c "$ROOT" \
        "$ROOT/scripts/run-busybee.sh; exec \$SHELL"
    tmux new-window -t "$SESSION" -n psql -c "$ROOT" \
        "psql service=hive; exec \$SHELL"
    tmux new-window -t "$SESSION" -n shell -c "$ROOT"
    tmux select-window -t "$SESSION:hive"
fi

if [ -t 1 ] && [ -z "${TMUX:-}" ]; then
    exec tmux attach -t "$SESSION"
fi
