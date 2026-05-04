#!/usr/bin/env bash
# Hive blue-green rollback.
#
# Brings the previously-deployed slot back online. Refuses to roll back if any
# new migrations were introduced between the rollback target's commit and the
# currently-active commit (rolling back a binary against a migrated DB is a
# good way to corrupt data). Pass --force to override.

set -euo pipefail

PROJECT_ROOT="/home/drone/hive"
BIN_DIR="$PROJECT_ROOT/bin"
UPSTREAM_FILE="/etc/nginx/conf.d/hive-upstream-server.conf"
HEALTH_TIMEOUT=60
DRAIN_SECONDS=10

FORCE=
[ "${1:-}" = "--force" ] && FORCE=1

cd "$PROJECT_ROOT"

# Detect active color from nginx (single source of truth — same as deploy.sh).
if [ ! -f "$UPSTREAM_FILE" ]; then
    echo "ERROR: $UPSTREAM_FILE missing. Nothing to roll back from." >&2
    exit 1
fi
UPSTREAM_PORT=$(grep -oE '127\.0\.0\.1:(3000|3001)' "$UPSTREAM_FILE" | head -1 | grep -oE '[0-9]+$' || true)
case "$UPSTREAM_PORT" in
    3000) ACTIVE=blue;  TARGET=green; TARGET_PORT=3001 ;;
    3001) ACTIVE=green; TARGET=blue;  TARGET_PORT=3000 ;;
    *)
        echo "ERROR: cannot parse upstream port from $UPSTREAM_FILE" >&2
        cat "$UPSTREAM_FILE" >&2
        exit 1
        ;;
esac
if ! sudo systemctl is-active --quiet "hive@$ACTIVE"; then
    echo "ERROR: nginx points to $ACTIVE but hive@$ACTIVE is not active — manual cleanup required." >&2
    exit 1
fi

TARGET_BIN="$BIN_DIR/hive-$TARGET"
TARGET_SHA_FILE="$BIN_DIR/hive-$TARGET.sha"
ACTIVE_SHA_FILE="$BIN_DIR/hive-$ACTIVE.sha"

if [ ! -x "$TARGET_BIN" ]; then
    echo "ERROR: rollback target binary not found at $TARGET_BIN." >&2
    echo "       (Have you deployed at least twice? Rollback only works one step back.)" >&2
    exit 1
fi

# Migration safety check
if [ -f "$TARGET_SHA_FILE" ] && [ -f "$ACTIVE_SHA_FILE" ]; then
    TARGET_SHA=$(cat "$TARGET_SHA_FILE")
    ACTIVE_SHA=$(cat "$ACTIVE_SHA_FILE")
    echo "→ Active: $ACTIVE @ ${ACTIVE_SHA:0:12}"
    echo "→ Target: $TARGET @ ${TARGET_SHA:0:12}"

    if MIGRATION_DIFF=$(git diff --name-only "$TARGET_SHA".."$ACTIVE_SHA" -- db/migrations/ 2>/dev/null) && [ -n "$MIGRATION_DIFF" ]; then
        echo "" >&2
        echo "ERROR: migrations were introduced between target and active:" >&2
        echo "$MIGRATION_DIFF" | sed 's/^/  /' >&2
        echo "" >&2
        echo "These migrations have already been applied to the database. Rolling back" >&2
        echo "to a binary that doesn't know about them risks data corruption." >&2
        if [ -z "$FORCE" ]; then
            echo "Pass --force to override (only if you know what you're doing)." >&2
            exit 1
        fi
        echo "→ --force given; proceeding anyway." >&2
    fi
else
    echo "WARNING: missing SHA file(s) for $ACTIVE or $TARGET — cannot verify migration safety." >&2
    if [ -z "$FORCE" ]; then
        echo "Pass --force to roll back anyway." >&2
        exit 1
    fi
fi

echo "→ Rolling back from $ACTIVE to $TARGET (port $TARGET_PORT)..."

sudo systemctl start "hive@$TARGET"

echo "→ Waiting for /health on :$TARGET_PORT (up to ${HEALTH_TIMEOUT}s)..."
for i in $(seq 1 "$HEALTH_TIMEOUT"); do
    if curl -sf "http://127.0.0.1:$TARGET_PORT/health" > /dev/null 2>&1; then
        echo "  liveness OK after ${i}s"
        break
    fi
    if [ "$i" -eq "$HEALTH_TIMEOUT" ]; then
        echo "ERROR: $TARGET failed liveness. Recent logs:" >&2
        sudo journalctl -u "hive@$TARGET" -n 50 --no-pager >&2 || true
        sudo systemctl stop "hive@$TARGET"
        exit 1
    fi
    sleep 1
done

READY_BODY=$(curl -sf "http://127.0.0.1:$TARGET_PORT/health/ready" 2>&1) || {
    echo "ERROR: $TARGET not ready: $READY_BODY" >&2
    sudo journalctl -u "hive@$TARGET" -n 50 --no-pager >&2 || true
    sudo systemctl stop "hive@$TARGET"
    exit 1
}
echo "  readiness OK ($READY_BODY)"

echo "server 127.0.0.1:$TARGET_PORT;" | sudo tee "$UPSTREAM_FILE" > /dev/null
sudo nginx -s reload
echo "→ nginx now routes to $TARGET (port $TARGET_PORT)"

echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
sleep "$DRAIN_SECONDS"

sudo systemctl stop "hive@$ACTIVE"
echo "→ Rollback complete. Active: $TARGET on port $TARGET_PORT."
