#!/usr/bin/env bash
set -euo pipefail

SESSION=hive
ROOT=/home/drone/hive

if ! tmux has-session -t "$SESSION" 2>/dev/null; then
    # Cron supplies /bin/sh and a minimal PATH, rather than the account's shell.
    SHELL=$(getent passwd "$(id -u)" | cut -d: -f7)
    export SHELL
    printf -v LOGIN_COMMAND 'exec %q -l' "$SHELL"

    pane_command() {
        printf '%q -lic %q' "$SHELL" "$1; $LOGIN_COMMAND"
    }

    tmux new-session -d -s "$SESSION" -n hive -c "$ROOT" \
        "$(pane_command 'journalctl -f -u hive@blue -u hive@green')"
    tmux set-option -t "$SESSION" default-shell "$SHELL"
    tmux new-window -t "$SESSION:" -n hydra -c "$ROOT" \
        "$(pane_command "$ROOT/scripts/run-hydra.sh")"
    tmux new-window -t "$SESSION:" -n busybee -c "$ROOT" \
        "$(pane_command "$ROOT/scripts/run-busybee.sh")"
    tmux new-window -t "$SESSION:" -n psql -c "$ROOT" \
        "$(pane_command 'for attempt in {1..60}; do pg_isready -q -d service=hive && break; sleep 1; done; psql service=hive')"
    tmux new-window -t "$SESSION:" -n puzzle-bot -c "$HOME/puzzle-bot" \
        "$(pane_command 'uv run python -m puzzle_bot')"
    tmux new-window -t "$SESSION:" -n shell -c "$ROOT" "$LOGIN_COMMAND"
    tmux select-window -t "$SESSION:hive"
fi

if [ -t 1 ] && [ -z "${TMUX:-}" ]; then
    exec tmux attach -t "$SESSION"
fi
