#!/usr/bin/env bash
set -uo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

BASE="${1:-https://hivegame.com}"
FAILS=0
pass() { printf 'PASS  %s\n' "$*"; }
warn() { printf 'WARN  %s\n' "$*"; }
fail() { printf 'FAIL  %s\n' "$*"; FAILS=$((FAILS + 1)); }

CURRENT=$(read_marker current)
ACTIVE=$(active_instance_slot)
if [ "$ACTIVE" = none ]; then
    fail "$ACTIVE_FILE names no slot; jobs and websockets are off everywhere"
    EXPECTED_ADDR=
else
    EXPECTED_ADDR="127.0.0.1:$(slot_port "$ACTIVE")"
fi
echo "expect release ${CURRENT:0:12} on ${ACTIVE} (${EXPECTED_ADDR:-?})"

health_of() {
    local response
    response=$(curl -s -i -m 5 "$1/health" 2>/dev/null | tr -d '\r') || true
    printf '%s %s\n' "$(printf '%s\n' "$response" | tail -1)" \
        "$(printf '%s\n' "$response" | grep -i '^x-hive-addr:' | awk '{print $2}' | head -1)"
}

for front in "$BASE" "$NGINX_LOCAL"; do
    seen=$(health_of "$front")
    if [ "$seen" = "$CURRENT $EXPECTED_ADDR" ]; then
        pass "$front/health -> ${CURRENT:0:12} from $ACTIVE"
    else
        fail "$front/health -> '$seen', expected '$CURRENT $EXPECTED_ADDR'"
    fi
done

PAGE=$(curl -s -m 10 -w '\n%{http_code}' "$BASE/") || true
if [ "$(printf '%s\n' "$PAGE" | tail -1)" = 200 ] && grep -q '<title>HiveGame.com' <<< "$PAGE"; then
    pass "$BASE/ renders"
else
    fail "$BASE/ did not render (HTTP $(printf '%s\n' "$PAGE" | tail -1))"
fi

BUNDLE=$(grep -oE '/pkg/HiveGame[^"]+\.wasm' <<< "$PAGE" | head -1)
if [ -z "$BUNDLE" ]; then
    fail "no /pkg/*.wasm referenced by $BASE/"
else
    HEADERS=$(curl -s -o /dev/null -D - -m 30 -H 'Accept-Encoding: gzip' "$BASE$BUNDLE" | tr -d '\r')
    grep -qE '^HTTP/[0-9.]+ 200' <<< "$HEADERS" && pass "$BUNDLE 200" || fail "$BUNDLE: $(head -1 <<< "$HEADERS")"
    grep -qi '^content-type: application/wasm' <<< "$HEADERS" && pass "wasm content-type" || fail "wasm served as: $(grep -i '^content-type' <<< "$HEADERS")"
    grep -qi '^content-encoding: gzip' <<< "$HEADERS" && pass "wasm gzip_static" || warn "wasm not gzip-encoded"
    grep -qi '^cache-control:.*immutable' <<< "$HEADERS" && pass "wasm cache immutable" || warn "wasm not marked immutable"
fi

WS_CODE=$(curl -s -o /dev/null -w '%{http_code}' -m 3 --http1.1 \
    -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' \
    -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' "$BASE/ws/" 2>/dev/null) || true
case "$WS_CODE" in
    101) pass "websocket upgrade via $BASE" ;;
    503) fail "websocket refused (503): the handoff to the active instance has not happened" ;;
    *)   fail "websocket upgrade returned '$WS_CODE'" ;;
esac

READY_CODE=$(curl -s -o /dev/null -w '%{http_code}' -m 5 "$BASE/health/ready") || true
[ "$READY_CODE" = 403 ] && pass "/health/ready is not public" || fail "/health/ready returned $READY_CODE publicly"

for port in 3000 3001 3999 8080; do
    exposed=$(ss -tln "sport = :$port" | awk 'NR>1 {print $4}' | grep -vE '^(127\.0\.0\.1|\[::1\]):' | tr '\n' ' ')
    [ -z "$exposed" ] && pass ":$port not public" || fail ":$port listens publicly on $exposed"
done

pgrep -f 'cargo-leptos leptos serve' > /dev/null && fail "the old cargo leptos serve process is still running" || pass "no old cargo leptos serve process"
pgrep -f 'release/hive-hydra' > /dev/null && pass "hive-hydra running" || warn "hive-hydra not running"
pgrep -f 'uvicorn api:app' > /dev/null && pass "busybee running" || warn "busybee not running"

echo
print_status
echo
echo "smoke: $FAILS failed"
[ "$FAILS" -eq 0 ]
