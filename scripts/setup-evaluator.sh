#!/usr/bin/env bash
# Sets up engine evals on the box; safe to re-run, every step skips what is already done.
# The engine is installed beforehand and described in ~/.config/hive-evaluator/engines.toml
# (hive-evaluator/README.md, Engines).
#   as drone:  scripts/setup-evaluator.sh
#   then once: sudo scripts/setup-evaluator.sh --install-token
set -euo pipefail

ROOT=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
# Fixed rather than $HOME: the root step has to find drone's file.
ENV_FILE=/home/drone/.config/hive-evaluator/env
ENGINES_FILE=/home/drone/.config/hive-evaluator/engines.toml
PROD_ENV=/etc/hive/prod.env

token_of() { sed -n 's/^EVAL_WORKER_TOKEN=//p' "$1" 2>/dev/null | tr -d '"' | head -1; }
step() { printf '\n== %s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

if [ "${1:-}" = --install-token ]; then
    [ "$(id -u)" = 0 ] || die "--install-token edits $PROD_ENV; run it with sudo"
    TOKEN=$(token_of "$ENV_FILE")
    [ -n "$TOKEN" ] || die "no token in $ENV_FILE yet; run this script as drone first"
    if [ "$(token_of "$PROD_ENV")" = "$TOKEN" ]; then
        echo "$PROD_ENV already has the evaluator token."
        exit 0
    fi
    tmp=$(mktemp)
    grep -v '^EVAL_WORKER_TOKEN=' "$PROD_ENV" > "$tmp" || true
    echo "EVAL_WORKER_TOKEN=$TOKEN" >> "$tmp"
    install -o root -g drone -m 640 "$tmp" "$PROD_ENV"
    rm -f "$tmp"
    echo "$PROD_ENV now has the evaluator token. The site reads it when a slot starts:"
    echo "the next scripts/deploy.sh turns evals on."
    exit 0
fi

[ "$(id -un)" = drone ] || die "run as drone (or with --install-token as root)"

step "engines in $ENGINES_FILE"
[ -r "$ENGINES_FILE" ] || die "no $ENGINES_FILE: install the engine, then describe it there (hive-evaluator/README.md, Engines)"
chmod 600 "$ENGINES_FILE"
echo "present, readable by drone only"

step "tools"
command -v openssl > /dev/null || die "missing openssl: sudo apt install openssl"
echo "openssl"

step "worker settings in $ENV_FILE"
umask 077
mkdir -p "$(dirname "$ENV_FILE")"
if [ ! -r "$ENV_FILE" ]; then
    cat > "$ENV_FILE" <<EOF
EVAL_WORKER_TOKEN=$(openssl rand -hex 32)
EVAL_WORKER_NAME=prod-1
# Starts with user requests only; set true to also evaluate games while nobody waits.
EVAL_AUTO=false
EOF
    echo "written, with a new token"
else
    echo "already there"
fi
chmod 600 "$ENV_FILE"

step "site token"
if [ "$(token_of "$PROD_ENV")" = "$(token_of "$ENV_FILE")" ]; then
    echo "$PROD_ENV has the same token"
    step "tmux"
    "$ROOT/scripts/tmux-session.sh" < /dev/null > /dev/null
    echo "the evaluator window runs in session 'hive'; scripts/smoke.sh should show it"
else
    cat <<EOF
The site does not have this token yet. Once, as a user with sudo:
    sudo $ROOT/scripts/setup-evaluator.sh --install-token
then deploy (scripts/deploy.sh) so the site reads it, and run this script again to start
the worker.
EOF
fi
