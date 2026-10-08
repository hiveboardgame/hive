#!/usr/bin/env bash
# Sets up engine evals on the box; safe to re-run, every step skips what is already done.
#   as drone:  scripts/setup-evaluator.sh [STOCKBEE_DIR]      (default ~/stockbee)
#   then once: sudo scripts/setup-evaluator.sh --install-token
set -euo pipefail

ROOT=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
# Fixed rather than $HOME: the root step has to find drone's file.
ENV_FILE=/home/drone/.config/hive-evaluator/env
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
SB=$(readlink -f "${1:-$HOME/stockbee}")

step "StockBee in $SB"
if [ ! -d "$SB" ]; then
    cat >&2 <<EOF
ERROR: no StockBee directory at $SB. Copy it from a machine that has it, without build
output (it is rebuilt here):
    rsync -a --exclude build --exclude out --exclude venv stockbee/ drone@<this box>:$SB/
EOF
    exit 1
fi
case "$SB/" in "$ROOT"/*) die "keep StockBee outside the checkout ($ROOT); it must never reach git" ;; esac
for f in CMakeLists.txt src/main.cpp src/graph_features.cpp tools/az_eval_server.py stockbee.pt; do
    [ -e "$SB/$f" ] || die "$SB/$f is missing"
done
chmod 700 "$SB"
echo "present, readable by drone only"
if grep -q 'weights_only=False' "$SB/tools/az_eval_server.py"; then
    # The net is a plain state dict; full unpickling would let a swapped file run code here.
    sed -i 's/weights_only=False/weights_only=True/g' "$SB/tools/az_eval_server.py"
    echo "switched the eval server to load the net as plain weights"
fi

step "tools"
missing=()
for tool in cmake g++ python3 openssl; do command -v "$tool" > /dev/null || missing+=("$tool"); done
python3 -c 'import venv, ensurepip' 2> /dev/null || missing+=(python3-venv)
[ ${#missing[@]} -eq 0 ] || die "missing ${missing[*]}: sudo apt install ${missing[*]}"
echo "cmake, g++, python3-venv, openssl"

engine_runs() { printf 'info\nexit\n' | "$SB/build/stockbee" 2> /dev/null | grep -q '^id name StockBee'; }
step "engine"
if [ -x "$SB/build/stockbee" ] && ! engine_runs; then
    echo "the existing build does not run here (copied from another machine?); rebuilding"
    rm -rf "$SB/build"
fi
if [ ! -x "$SB/build/stockbee" ] \
    || [ -n "$(find "$SB/src" "$SB/CMakeLists.txt" -newer "$SB/build/stockbee" -print -quit)" ]; then
    echo "building (a few minutes, at nice 15)"
    nice -n 15 cmake -S "$SB" -B "$SB/build" -DCMAKE_BUILD_TYPE=Release > "$SB/build.log" 2>&1
    nice -n 15 cmake --build "$SB/build" -j2 --target stockbee >> "$SB/build.log" 2>&1 \
        || { tail -30 "$SB/build.log"; die "engine build failed; full log in $SB/build.log"; }
fi
if [ ! -r "$SB/build/libgraph_features.so" ] \
    || [ "$SB/src/graph_features.cpp" -nt "$SB/build/libgraph_features.so" ]; then
    nice -n 15 g++ -O2 -shared -fPIC -std=c++17 -fopenmp "$SB/src/graph_features.cpp" \
        -o "$SB/build/libgraph_features.so"
fi
engine_runs || die "$SB/build/stockbee does not start"
echo "built and starts"

step "python"
if ! "$SB/venv/bin/python" -c 'import torch, numpy' 2> /dev/null; then
    echo "creating $SB/venv with CPU-only torch (about 200 MB download)"
    [ -x "$SB/venv/bin/python" ] || python3 -m venv "$SB/venv"
    "$SB/venv/bin/pip" install -q --upgrade pip
    "$SB/venv/bin/pip" install -q numpy
    "$SB/venv/bin/pip" install -q torch --index-url https://download.pytorch.org/whl/cpu
fi
"$SB/venv/bin/python" -c 'import torch, numpy' || die "the venv cannot import torch and numpy"
echo "$SB/venv has torch and numpy"

step "worker settings in $ENV_FILE"
umask 077
mkdir -p "$(dirname "$ENV_FILE")"
if [ ! -r "$ENV_FILE" ]; then
    cat > "$ENV_FILE" <<EOF
EVAL_WORKER_TOKEN=$(openssl rand -hex 32)
STOCKBEE_DIR=$SB
STOCKBEE_PYTHON=$SB/venv/bin/python
EVAL_WORKER_NAME=prod-1
# Starts with user requests only; set true to also evaluate strong games while nobody waits.
EVAL_AUTO=false
EOF
    echo "written, with a new token"
else
    grep -q "^STOCKBEE_DIR=$SB\$" "$ENV_FILE" \
        || echo "kept as it is; note it points at $(sed -n 's/^STOCKBEE_DIR=//p' "$ENV_FILE"), not $SB"
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
