#!/usr/bin/env bash
set -uo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

FAILS=0
WARNS=0
pass() { printf 'PASS  %s\n' "$*"; }
warn() { printf 'WARN  %s\n' "$*"; WARNS=$((WARNS + 1)); }
fail() { printf 'FAIL  %s\n' "$*"; FAILS=$((FAILS + 1)); }
check() {
    local label="$1"
    shift
    if "$@" > /dev/null 2>&1; then pass "$label"; else fail "$label"; fi
}
others_can_enter() { [ "$(stat -c %A "$1" | cut -c10)" = x ]; }
public_binds() { ss -tln "sport = :$1" | awk 'NR>1 {print $4}' | grep -vE '^(127\.0\.0\.1|\[::1\]):' || true; }

cd "$PROJECT_ROOT" || exit 1
HEAD_SHA=$(git rev-parse HEAD)

echo "== checkout"
[ "$(id -un)" = drone ] && pass "running as drone" || fail "running as $(id -un), not drone"
echo "      branch $(git symbolic-ref --quiet --short HEAD || echo DETACHED) at ${HEAD_SHA:0:12}"
DIRTY=$(git status --porcelain --untracked-files=no)
if [ -z "$DIRTY" ]; then pass "no tracked changes"; else warn "tracked changes: $(echo "$DIRTY" | tr '\n' ' ')"; fi

echo "== tools"
for tool in git curl ss flock psql pg_dump cargo cargo-leptos tmux; do
    check "$tool installed" command -v "$tool"
done

echo "== database access (service=$PG_SERVICE)"
if [ "$(psql "service=$PG_SERVICE" -XAtqc 'select 1' 2>&1)" = 1 ]; then
    pass "psql service=$PG_SERVICE connects"
else
    fail "psql service=$PG_SERVICE does not connect (~/.pg_service.conf, ~/.pgpass)"
fi
[ "$(stat -c %a "$HOME/.pgpass" 2>/dev/null)" = 600 ] && pass "~/.pgpass is 0600" || fail "~/.pgpass missing or not 0600"
if [ -r /etc/hive/prod.env ]; then
    APP_ROLE=$(sed -nE 's#^DATABASE_URL="?postgres(ql)?://([^:@/]+).*#\2#p' /etc/hive/prod.env | head -1)
    SVC_ROLE=$(psql "service=$PG_SERVICE" -XAtqc 'select current_user' 2>/dev/null)
    [ -n "$APP_ROLE" ] && [ "$APP_ROLE" = "$SVC_ROLE" ] && pass "service=$PG_SERVICE connects as the app's role ($APP_ROLE)" \
        || fail "service=$PG_SERVICE connects as '$SVC_ROLE', the app as '$APP_ROLE'; migrations must run as the app's role"
fi

echo "== /etc/hive"
for f in prod.env common.env blue.env green.env; do
    check "/etc/hive/$f exists" test -r "/etc/hive/$f"
done
if [ -r /etc/hive/prod.env ]; then
    [ "$(stat -c '%U:%G %a' /etc/hive/prod.env)" = "root:drone 640" ] && pass "prod.env is root:drone 0640" \
        || warn "prod.env is $(stat -c '%U:%G %a' /etc/hive/prod.env), expected root:drone 640"
    for key in DATABASE_URL COOKIE_SECRET_KEY JWT_SECRET_KEY VAPID_PRIVATE_KEY VAPID_SUBJECT LETTERMINT_API_KEY; do
        check "prod.env sets $key" grep -q "^$key=" /etc/hive/prod.env
    done
    grep -q '^EVAL_WORKER_TOKEN=.' /etc/hive/prod.env && pass "prod.env sets EVAL_WORKER_TOKEN" \
        || warn "prod.env has no EVAL_WORKER_TOKEN: engine evals are off (scripts/README.md, Evals)"
    if grep -E '^[A-Z_]+=' /etc/hive/prod.env | cut -d= -f2- | grep -q '[$\\]'; then
        warn "a prod.env value contains \$ or \\: systemd reads it literally, dotenvy did not; check it"
    else
        pass "prod.env values parse the same under systemd and dotenvy"
    fi
fi
check "common.env sets HIVE_ACTIVE_FILE" grep -q '^HIVE_ACTIVE_FILE=' /etc/hive/common.env
check "systemd knows hive@.service" systemctl cat hive@blue.service

echo "== sudo (no password)"
SUDO_RULES=$(sudo -n -l 2>/dev/null)
if [ -z "$SUDO_RULES" ]; then
    fail "sudo -n -l needs a password or failed; sudoers rule not installed"
else
    for cmd in "systemctl start hive@blue" "systemctl stop hive@green" "systemctl enable hive@green" \
               "systemctl disable hive@blue" "journalctl -u hive@green -n 50 --no-pager" \
               "/usr/local/sbin/hive-upstream blue" "/usr/local/sbin/hive-upstream green" "nginx -t"; do
        check "sudo allows: $cmd" grep -qF "$cmd" <<< "$SUDO_RULES"
    done
fi

echo "== nginx"
[ "$(stat -c '%U %a' "$UPSTREAM_HELPER" 2>/dev/null)" = "root 755" ] && pass "$UPSTREAM_HELPER installed, root-owned" || fail "$UPSTREAM_HELPER missing or not root 0755 (cutover step 2)"
if sudo -n nginx -t > /dev/null 2>&1; then pass "nginx -t"; else fail "nginx -t fails, or sudo is not set up yet"; fi
SITE=/etc/nginx/sites-available/default
grep -q 'include /etc/nginx/hive-upstream.conf' "$SITE" && pass "site includes the upstream file" || warn "site has no hive_active upstream yet (cutover step 3)"
grep -q 'listen 127.0.0.1:3999' "$SITE" && pass "site listens on 127.0.0.1:3999" || warn "site has no 3999 listener yet (cutover step 3)"
grep -q 'try_files /blue/site' "$SITE" && pass "site serves /pkg/ from both slots" || warn "site has no /pkg/ block yet (cutover step 3)"
grep -q '@app' "$SITE" && pass "/pkg/ falls back to the app" || warn "site has no @app fallback for /pkg/ yet (cutover step 3)"
grep -q 'location /api/v1/evals/' "$SITE" && pass "site keeps the eval worker API off the internet" || warn "site does not block /api/v1/evals/ publicly yet; see scripts/nginx/sites-enabled-default.example"
check "mime.types knows application/wasm" grep -q 'application/wasm' /etc/nginx/mime.types
/usr/sbin/nginx -V 2>&1 | grep -q http_gzip_static_module && pass "nginx has gzip_static" || fail "nginx lacks gzip_static"
for dir in /home/drone "$PROJECT_ROOT"; do
    others_can_enter "$dir" && pass "www-data can enter $dir" || fail "www-data cannot enter $dir"
done

echo "== on-box services"
systemctl is-active --quiet cron && pass "cron runs (@reboot brings tmux back)" || fail "cron is not running; the @reboot tmux session will not start"
if ! git -C "$PROJECT_ROOT" diff --quiet -- .env; then
    [ "$(stat -c %a "$PROJECT_ROOT/.env" 2>/dev/null)" = 600 ] && pass ".env holds local changes and is 0600" \
        || warn ".env holds local changes (prod values?) and is readable by others: chmod 600 .env"
fi
id -nG | grep -qw systemd-journal && pass "drone is in systemd-journal" || warn "drone not in systemd-journal (or needs a new login)"
check "~/.config/hive-hydra/env exists" test -r "$HOME/.config/hive-hydra/env"
grep -q '^HIVE_HYDRA_BASE_URL=http://localhost:3999' "$HOME/.config/hive-hydra/env" 2>/dev/null \
    && pass "hydra uses localhost:3999" || warn "hydra env has no HIVE_HYDRA_BASE_URL=http://localhost:3999 (cutover step 4)"
check "~/.config/busybee/env exists" test -r "$HOME/.config/busybee/env"
check "busybee venv exists" test -r "$PROJECT_ROOT/busybee/venv/bin/activate"
EVAL_ENV="$HOME/.config/hive-evaluator/env"
if [ -r "$EVAL_ENV" ]; then
    pass "~/.config/hive-evaluator/env exists"
    # A subshell, so the evaluator's settings never leak into the checks after this one.
    read -r SB_DIR SB_NET SB_PY < <(set -a; . "$EVAL_ENV"; echo "${STOCKBEE_DIR:-} ${STOCKBEE_NET:-stockbee.pt} ${STOCKBEE_PYTHON:-python3}")
    token_hash() { sed -n "s/^EVAL_WORKER_TOKEN=//p" "$1" 2>/dev/null | tr -d '"' | sha256sum | cut -c1-16; }
    if [ -r /etc/hive/prod.env ] && [ "$(token_hash "$EVAL_ENV")" = "$(token_hash /etc/hive/prod.env)" ]; then
        pass "evaluator token matches prod.env"
    else
        warn "evaluator token is missing or differs from prod.env's EVAL_WORKER_TOKEN"
    fi
    if [ -n "$SB_DIR" ]; then
        check "StockBee engine built" test -x "$SB_DIR/build/stockbee"
        check "StockBee featurizer built" test -r "$SB_DIR/build/libgraph_features.so"
        check "StockBee net present" test -r "$SB_DIR/$SB_NET"
        check "StockBee eval server present" test -r "$SB_DIR/tools/az_eval_server.py"
        grep -q 'weights_only=False' "$SB_DIR/tools/az_eval_server.py" 2>/dev/null \
            && fail "az_eval_server.py loads the net with weights_only=False: a swapped net could run code" \
            || pass "the eval server loads the net as plain weights"
        check "evaluator python has torch and numpy" "$SB_PY" -c 'import torch, numpy'
    else
        warn "evaluator env sets no STOCKBEE_DIR"
    fi
else
    warn "no ~/.config/hive-evaluator/env: engine evals are off (scripts/README.md, Evals)"
fi

echo "== disk"
ROOT_FREE=$(df --output=avail -BG / | tail -1 | tr -dc 0-9)
[ "$ROOT_FREE" -ge 10 ] && pass "/ has ${ROOT_FREE}G free" || fail "/ has only ${ROOT_FREE}G free"

echo "== release ${HEAD_SHA:0:12}"
if release_complete "$HEAD_SHA"; then
    pass "release built"
    if PENDING=$(DATABASE_URL="service=$PG_SERVICE" "$(release_dir "$HEAD_SHA")/hive" --pending-migrations 2>&1); then
        pass "pending migrations readable"
        printf '%s\n' "$PENDING" | sed 's/^/      /'
    else
        fail "cannot list pending migrations: $PENDING"
    fi
else
    warn "release not built yet; run scripts/deploy.sh --build-only"
fi
SCHEMA_OUT=$(mktemp)
RC=0
check_schema_compat "$HEAD_SHA" > "$SCHEMA_OUT" 2>&1 || RC=$?
sed 's/^/      /' "$SCHEMA_OUT"
case "$RC" in
    0) pass "${HEAD_SHA:0:12} fits the applied schema" ;;
    1) fail "${HEAD_SHA:0:12} may not fit the applied schema" ;;
    *) fail "cannot judge the schema for ${HEAD_SHA:0:12}" ;;
esac
LIVE=$(read_marker current)
if [ -z "$LIVE" ]; then
    LIVE=$(cat "$HOME/old-release.sha" 2>/dev/null || true)
    if [ -n "$LIVE" ] && git cat-file -e "$LIVE^{commit}" 2>/dev/null; then
        pass "old process commit recorded: ${LIVE:0:12} (~/old-release.sha)"
    else
        fail "~/old-release.sha missing or not a commit; record it before the first pull"
        LIVE=
    fi
fi
if [ -n "$LIVE" ] && [ "$LIVE" != "$HEAD_SHA" ]; then
    RC=0
    check_schema_compat "$LIVE" "$HEAD_SHA" > "$SCHEMA_OUT" 2>&1 || RC=$?
    sed 's/^/      /' "$SCHEMA_OUT"
    case "$RC" in
        0) pass "live ${LIVE:0:12} fits the schema after migrating" ;;
        1) fail "live ${LIVE:0:12} may not fit the schema after migrating" ;;
        *) fail "cannot judge the schema for live ${LIVE:0:12}" ;;
    esac
fi
rm -f "$SCHEMA_OUT"

echo "== ports"
for port in 3000 3001 3999 8080; do
    binds=$(ss -tln "sport = :$port" | awk 'NR>1 {print $4}' | tr '\n' ' ')
    exposed=$(public_binds "$port" | tr '\n' ' ')
    if [ -n "$exposed" ]; then
        warn ":$port listens publicly on $exposed"
    else
        pass ":$port ${binds:-closed}"
    fi
done

echo "== status"
print_status

echo
echo "preflight: $FAILS failed, $WARNS warnings"
[ "$FAILS" -eq 0 ]
