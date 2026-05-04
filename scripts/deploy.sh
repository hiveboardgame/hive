#!/usr/bin/env bash
# Hive blue-green production deploy.
#
# End-to-end: pulls latest code, refreshes .env, builds the release binary,
# then swaps the active systemd slot behind nginx with no HTTP downtime.
# WebSocket clients reconnect automatically (~1-3s spinner).

set -euo pipefail

PROJECT_ROOT="/home/drone/hive"
ENV_SOURCE="/home/leex/hive/.env"
BINARY="$PROJECT_ROOT/.cargo/target/release/apis"
BIN_DIR="$PROJECT_ROOT/bin"
UPSTREAM_FILE="/etc/nginx/conf.d/hive-upstream-server.conf"
DRAIN_SECONDS=10
HEALTH_TIMEOUT=60

cd "$PROJECT_ROOT"

echo "→ Pulling latest..."
git pull

if [ -f "$ENV_SOURCE" ]; then
    echo "→ Refreshing .env from $ENV_SOURCE"
    cp "$ENV_SOURCE" .env
fi

echo "→ Building release binary..."
LEPTOS_HASH_FILES=true nix develop -c cargo leptos build -rP

if [ ! -x "$BINARY" ]; then
    echo "ERROR: build did not produce $BINARY" >&2
    exit 1
fi

# Detect active color
if sudo systemctl is-active --quiet hive@blue; then
    ACTIVE=blue;  IDLE=green; IDLE_PORT=3001
elif sudo systemctl is-active --quiet hive@green; then
    ACTIVE=green; IDLE=blue;  IDLE_PORT=3000
else
    # First-time bootstrap: nothing running, default to blue
    ACTIVE=none;  IDLE=blue;  IDLE_PORT=3000
fi

echo "→ Active: $ACTIVE; deploying to: $IDLE (port $IDLE_PORT)"

mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/hive-$IDLE"
chmod +x "$BIN_DIR/hive-$IDLE"
git rev-parse HEAD > "$BIN_DIR/hive-$IDLE.sha"

sudo systemctl stop "hive@$IDLE" 2>/dev/null || true
sudo systemctl start "hive@$IDLE"

echo "→ Waiting for /health on :$IDLE_PORT (up to ${HEALTH_TIMEOUT}s)..."
for i in $(seq 1 "$HEALTH_TIMEOUT"); do
    if curl -sf "http://127.0.0.1:$IDLE_PORT/health" > /dev/null 2>&1; then
        echo "  healthy after ${i}s"
        break
    fi
    if [ "$i" -eq "$HEALTH_TIMEOUT" ]; then
        echo "ERROR: $IDLE failed to become healthy. Recent logs:" >&2
        sudo journalctl -u "hive@$IDLE" -n 50 --no-pager >&2 || true
        sudo systemctl stop "hive@$IDLE"
        exit 1
    fi
    sleep 1
done

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
