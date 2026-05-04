#!/usr/bin/env bash
# Hive blue-green production deploy.
#
# End-to-end: detects which slot nginx is currently routing to, pulls latest
# code, lints new migrations for backward-incompat changes, builds the
# release binary AND assets into a per-slot site dir, then swaps the active
# systemd slot behind nginx with no HTTP downtime. WebSocket clients
# reconnect automatically (~1-3s spinner).

set -euo pipefail

PROJECT_ROOT="/home/drone/hive"
ENV_SOURCE="/home/leex/hive/.env"
BINARY="$PROJECT_ROOT/.cargo/target/release/apis"
SITE_BUILD_DIR="$PROJECT_ROOT/target/site"
BIN_DIR="$PROJECT_ROOT/bin"
UPSTREAM_FILE="/etc/nginx/conf.d/hive-upstream-server.conf"
DRAIN_SECONDS=10
HEALTH_TIMEOUT=60

cd "$PROJECT_ROOT"

# --- Detect active slot from nginx, not systemd ---------------------------
# Single source of truth: whatever nginx is sending traffic to is "active".
# Falling back to systemd here is what made the previous version unsafe after
# an interrupted deploy left both services up.
ACTIVE=none; ACTIVE_PORT=; IDLE=blue; IDLE_PORT=3000
if [ -f "$UPSTREAM_FILE" ]; then
    UPSTREAM_PORT=$(grep -oE '127\.0\.0\.1:(3000|3001)' "$UPSTREAM_FILE" | head -1 | grep -oE '[0-9]+$' || true)
    case "$UPSTREAM_PORT" in
        3000) ACTIVE=blue;  ACTIVE_PORT=3000; IDLE=green; IDLE_PORT=3001 ;;
        3001) ACTIVE=green; ACTIVE_PORT=3001; IDLE=blue;  IDLE_PORT=3000 ;;
        "")
            echo "WARNING: $UPSTREAM_FILE present but no recognised upstream — treating as bootstrap." >&2
            ;;
        *)
            echo "ERROR: unrecognised upstream port '$UPSTREAM_PORT' in $UPSTREAM_FILE" >&2
            cat "$UPSTREAM_FILE" >&2
            exit 1
            ;;
    esac
fi

# Sanity: if nginx points at a color, that color's service must actually be
# running. Otherwise we'd cheerfully deploy to the "idle" slot — which is the
# one nginx is *not* talking to — and never realise traffic is already broken.
if [ "$ACTIVE" != "none" ]; then
    if ! sudo systemctl is-active --quiet "hive@$ACTIVE"; then
        echo "ERROR: nginx upstream points to $ACTIVE (port $ACTIVE_PORT) but hive@$ACTIVE is not active." >&2
        echo "       Manual cleanup required — refusing to auto-recover." >&2
        exit 1
    fi
fi

echo "→ Active: $ACTIVE; deploying to: $IDLE (port $IDLE_PORT)"

echo "→ Pulling latest..."
PRE_PULL_SHA=$(git rev-parse HEAD)
git pull
POST_PULL_SHA=$(git rev-parse HEAD)

if [ -f "$ENV_SOURCE" ]; then
    echo "→ Refreshing .env from $ENV_SOURCE"
    cp "$ENV_SOURCE" .env
fi

# --- Migration safety lint ------------------------------------------------
# Migrations run on app boot (apis/src/main.rs). During the swap the OLD
# binary keeps serving against the NEW schema, so anything destructive here
# breaks live traffic. Refuse unless explicitly overridden.
ACTIVE_SHA_FILE="$BIN_DIR/hive-$ACTIVE.sha"
COMPARE_FROM=""
if [ "$ACTIVE" != "none" ] && [ -f "$ACTIVE_SHA_FILE" ]; then
    COMPARE_FROM=$(cat "$ACTIVE_SHA_FILE")
elif [ "$PRE_PULL_SHA" != "$POST_PULL_SHA" ]; then
    COMPARE_FROM="$PRE_PULL_SHA"
fi
if [ -n "$COMPARE_FROM" ]; then
    NEW_MIGRATION_FILES=$(git diff --name-only --diff-filter=AM "$COMPARE_FROM"..HEAD -- 'db/migrations/' 2>/dev/null | grep -E '/up\.sql$' || true)
    if [ -n "$NEW_MIGRATION_FILES" ]; then
        echo "→ New migrations since active deploy:"
        echo "$NEW_MIGRATION_FILES" | sed 's/^/    /'
        DESTRUCTIVE=$(echo "$NEW_MIGRATION_FILES" | xargs -r grep -liE \
            'DROP[[:space:]]+(COLUMN|TABLE|INDEX|CONSTRAINT|TYPE|SEQUENCE)|ALTER[[:space:]]+(TABLE|COLUMN)[[:space:]].+(RENAME|TYPE|SET[[:space:]]+NOT[[:space:]]+NULL)|TRUNCATE' \
            2>/dev/null || true)
        if [ -n "$DESTRUCTIVE" ]; then
            echo "" >&2
            echo "ERROR: migration(s) appear non-backward-compatible:" >&2
            echo "$DESTRUCTIVE" | sed 's/^/    /' >&2
            echo "" >&2
            echo "Blue/green needs the OLD binary to keep working against the NEW schema." >&2
            echo "If this really is safe (e.g. dropping a column nothing reads), set" >&2
            echo "ALLOW_DESTRUCTIVE_MIGRATION=1 and re-run." >&2
            if [ "${ALLOW_DESTRUCTIVE_MIGRATION:-}" != "1" ]; then
                exit 1
            fi
            echo "→ ALLOW_DESTRUCTIVE_MIGRATION=1; proceeding." >&2
        fi
    fi
fi

echo "→ Building release binary + assets..."
LEPTOS_HASH_FILES=true cargo leptos build -rP

if [ ! -x "$BINARY" ]; then
    echo "ERROR: build did not produce $BINARY" >&2
    exit 1
fi
if [ ! -d "$SITE_BUILD_DIR/pkg" ]; then
    echo "ERROR: build did not produce $SITE_BUILD_DIR/pkg" >&2
    exit 1
fi

# --- Stage release into per-slot dirs -------------------------------------
# Each slot gets its own immutable copy of the binary and asset bundle, so
# the live slot's hashed assets aren't trampled mid-deploy and rollback
# really restores a self-consistent release.
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/hive-$IDLE"
chmod +x "$BIN_DIR/hive-$IDLE"
git rev-parse HEAD > "$BIN_DIR/hive-$IDLE.sha"

SITE_DEST="$BIN_DIR/hive-$IDLE-site"
SITE_NEW="$SITE_DEST.new"
SITE_OLD="$SITE_DEST.old"
rm -rf "$SITE_NEW" "$SITE_OLD"
cp -a "$SITE_BUILD_DIR" "$SITE_NEW"
[ -d "$SITE_DEST" ] && mv "$SITE_DEST" "$SITE_OLD"
mv "$SITE_NEW" "$SITE_DEST"
rm -rf "$SITE_OLD"

sudo systemctl stop "hive@$IDLE" 2>/dev/null || true
sudo systemctl start "hive@$IDLE"

echo "→ Waiting for /health on :$IDLE_PORT (up to ${HEALTH_TIMEOUT}s)..."
for i in $(seq 1 "$HEALTH_TIMEOUT"); do
    if curl -sf "http://127.0.0.1:$IDLE_PORT/health" > /dev/null 2>&1; then
        echo "  liveness OK after ${i}s"
        break
    fi
    if [ "$i" -eq "$HEALTH_TIMEOUT" ]; then
        echo "ERROR: $IDLE failed liveness. Recent logs:" >&2
        sudo journalctl -u "hive@$IDLE" -n 50 --no-pager >&2 || true
        sudo systemctl stop "hive@$IDLE"
        exit 1
    fi
    sleep 1
done

# Deeper checks: DB reachable + per-slot assets on disk + SSR root renders.
READY_BODY=$(curl -sf "http://127.0.0.1:$IDLE_PORT/health/ready" 2>&1) || {
    echo "ERROR: $IDLE not ready: $READY_BODY" >&2
    sudo journalctl -u "hive@$IDLE" -n 50 --no-pager >&2 || true
    sudo systemctl stop "hive@$IDLE"
    exit 1
}
echo "  readiness OK ($READY_BODY)"

if ! curl -sf -o /dev/null "http://127.0.0.1:$IDLE_PORT/"; then
    echo "ERROR: $IDLE / did not return 200" >&2
    sudo systemctl stop "hive@$IDLE"
    exit 1
fi
echo "  SSR root OK"

echo "server 127.0.0.1:$IDLE_PORT;" | sudo tee "$UPSTREAM_FILE" > /dev/null
sudo nginx -s reload
echo "→ nginx now routes to $IDLE (port $IDLE_PORT)"

if [ "$ACTIVE" = "none" ]; then
    echo "→ Bootstrap complete. Serving from $IDLE."
    exit 0
fi

echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
sleep "$DRAIN_SECONDS"

sudo systemctl stop "hive@$ACTIVE"
echo "→ Done. Active: $IDLE on port $IDLE_PORT."
