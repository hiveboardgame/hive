#!/usr/bin/env bash
set -euo pipefail

SESSION=hive
ROOT=/home/drone/hive

# Run again on a live session to add a window that is missing, such as a newly added service.
ensure_window() {
    local name="$1"
    shift
    tmux list-windows -t "$SESSION" -F '#W' | grep -qx "$name" && return
    tmux new-window -d -t "$SESSION:" -n "$name" -c "$ROOT" "$@"
}

NEW_SESSION=
if ! tmux has-session -t "$SESSION" 2>/dev/null; then
    tmux new-session -d -s "$SESSION" -n hive -c "$ROOT" \
        "journalctl -f -u hive@blue -u hive@green; exec \$SHELL"
    NEW_SESSION=1
fi
ensure_window hydra "$ROOT/scripts/run-hydra.sh; exec \$SHELL"
ensure_window busybee "$ROOT/scripts/run-busybee.sh; exec \$SHELL"
# Only once scripts/setup-evaluator.sh has run; until then the window could only fail.
if [ -r "$HOME/.config/hive-evaluator/env" ]; then
    ensure_window evaluator "$ROOT/scripts/run-evaluator.sh; exec \$SHELL"
fi
ensure_window psql "psql service=hive; exec \$SHELL"
ensure_window shell
[ -n "$NEW_SESSION" ] && tmux select-window -t "$SESSION:hive"

if [ -t 1 ] && [ -z "${TMUX:-}" ]; then
    exec tmux attach -t "$SESSION"
fi
