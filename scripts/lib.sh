# shellcheck shell=bash
# Shared helpers for deploy.sh and rollback.sh. Sourced, not executed.
# shellcheck disable=SC2034  # consumed by the scripts that source this

PROJECT_ROOT="/home/drone/hive"
BIN_DIR="$PROJECT_ROOT/bin"
# One directory per slot: leptos resolves the hash manifest as
# `current_exe().parent()/hash.txt`, so two binaries sharing a parent would read
# — and each deploy would overwrite — one another's manifest.
slot_dir() { echo "$BIN_DIR/$1"; }

# Two deploys at once would interleave staging, both flip nginx, and each stop
# the slot the other just brought up. Held for the life of the script via the
# open descriptor, so it is released even on SIGKILL.
take_deploy_lock() {
    # `flock -n` exits 1 when the lock is held but 127 when the binary is
    # missing, and reporting both as "already running" would send you hunting
    # for a deploy that does not exist.
    command -v flock > /dev/null || {
        echo "ERROR: flock not found (Debian: apt install util-linux)." >&2
        exit 1
    }
    exec 9>"$PROJECT_ROOT/.deploy.lock"
    if ! flock -n 9; then
        echo "ERROR: another deploy or rollback is already running." >&2
        exit 1
    fi
}

# A page served by the outgoing slot moments before the flip fetches its hashed
# bundle moments after it, by which time nginx points at the incoming one.
# Without that release's assets the fetch 404s and the tab is dead. Driven by
# the source slot's own manifest, so this carries one generation rather than
# the accumulated union of every release that ever ran.
carry_over_assets() {
    local from_dir="$1" into_pkg="$2" f
    [ -f "$from_dir/pkg.manifest" ] || return 0
    [ -d "$from_dir/site/pkg" ] || return 0
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        [ -e "$into_pkg/$f" ] && continue
        mkdir -p "$(dirname "$into_pkg/$f")"
        cp -a "$from_dir/site/pkg/$f" "$into_pkg/$f" 2>/dev/null || true
    done < "$from_dir/pkg.manifest"
}

# Deliberately NOT under conf.d/: Debian and Ubuntu ship `include
# /etc/nginx/conf.d/*.conf;` inside `http { }`, so a file there holding a bare
# `server 127.0.0.1:3000;` is also parsed at http scope, where it is not a valid
# directive — nginx -t fails and the reload does nothing.
UPSTREAM_FILE="/etc/nginx/hive-upstream.conf"
PROD_ENV="/etc/hive/prod.env"
# Every socket still on the old slot is a client that cannot see events raised
# on the new one, so this window is a correctness cost, not just a courtesy.
# Kept only long enough for in-flight SSR requests, which are sub-second.
DRAIN_SECONDS=3
HEALTH_TIMEOUT=60
# Without these a slot that accepts the connection and then says nothing hangs
# curl forever, and the retry budget above never advances.
LIVENESS_PROBE_TIMEOUT=3
READY_PROBE_TIMEOUT=15

# Reads the colour nginx is currently routing to. Single source of truth:
# systemd can have both slots up after an interrupted deploy, nginx cannot.
# Echoes "<colour> <port>", or "none" when there is no usable upstream file.
read_active_slot() {
    [ -f "$UPSTREAM_FILE" ] || { echo none; return 0; }
    local port
    port=$(grep -oE '127\.0\.0\.1:(3000|3001)' "$UPSTREAM_FILE" | head -1 | grep -oE '[0-9]+$' || true)
    case "$port" in
        3000) echo "blue 3000" ;;
        3001) echo "green 3001" ;;
        "")   echo none ;;
        *)
            echo "ERROR: unrecognised upstream port '$port' in $UPSTREAM_FILE" >&2
            cat "$UPSTREAM_FILE" >&2
            return 1
            ;;
    esac
}

# Blocks until the slot answers /health, then checks /health/ready and the SSR
# root. Stops the slot and returns 1 on any failure, so the caller can abort
# with the live slot untouched.
#
# `curl -f` is deliberately not used for the readiness probe: it discards the
# response body, which is the only place health_ready says *what* is wrong.
verify_slot() {
    local colour="$1" port="$2" status body

    local deadline
    deadline=$(( $(date +%s) + HEALTH_TIMEOUT ))
    echo "→ Waiting for /health on :$port (up to ${HEALTH_TIMEOUT}s)..."
    while ! curl -sf -m "$LIVENESS_PROBE_TIMEOUT" "http://127.0.0.1:$port/health" > /dev/null 2>&1; do
        if [ "$(date +%s)" -ge "$deadline" ]; then
            echo "ERROR: $colour failed liveness within ${HEALTH_TIMEOUT}s. Recent logs:" >&2
            sudo journalctl -u "hive@$colour" -n 50 --no-pager >&2 || true
            sudo systemctl stop "hive@$colour"
            return 1
        fi
        sleep 1
    done
    echo "  liveness OK"

    # Loopback only. If /etc/hive/<colour>.env is missing or stale the binary
    # falls back to the site-addr baked into apis/Cargo.toml, which is
    # 0.0.0.0 — that puts the app on the public internet on this port, in
    # cleartext, reachable around nginx. There is no firewall behind this.
    local binds addr
    binds=$(ss -tln "sport = :$port" | awk 'NR>1 {print $4}')
    if [ -z "$binds" ]; then
        echo "ERROR: $colour answered /health but nothing is listening on :$port" >&2
        sudo systemctl stop "hive@$colour"
        return 1
    fi
    while read -r addr; do
        case "${addr%:*}" in
            127.0.0.1|"[::1]") ;;
            *)
                echo "ERROR: $colour is bound to $addr, not loopback." >&2
                echo "       Check LEPTOS_SITE_ADDR in /etc/hive/$colour.env." >&2
                sudo systemctl stop "hive@$colour"
                return 1
                ;;
        esac
    done <<< "$binds"
    echo "  bound to loopback only"

    body=$(curl -s -m "$READY_PROBE_TIMEOUT" -w '\n%{http_code}' \
        "http://127.0.0.1:$port/health/ready" 2>&1) || true
    status=$(printf '%s' "$body" | tail -1)
    body=$(printf '%s' "$body" | sed '$d')
    if [ "$status" != "200" ]; then
        echo "ERROR: $colour not ready (HTTP $status): $body" >&2
        sudo journalctl -u "hive@$colour" -n 50 --no-pager >&2 || true
        sudo systemctl stop "hive@$colour"
        return 1
    fi
    echo "  readiness OK ($body)"

    if ! curl -sf -m "$READY_PROBE_TIMEOUT" -o /dev/null "http://127.0.0.1:$port/"; then
        echo "ERROR: $colour / did not return 200" >&2
        sudo journalctl -u "hive@$colour" -n 50 --no-pager >&2 || true
        sudo systemctl stop "hive@$colour"
        return 1
    fi
    echo "  SSR root OK"
}

# Points nginx at $2 and reloads. `nginx -s reload` exits 0 even when the new
# config is rejected — the master just keeps the old one — so a bad upstream
# file would flip nothing, and the caller would then stop the slot that is
# still serving. Validate first, and put the old file back if it fails.
swap_upstream() {
    local colour="$1" port="$2" backup had_file=0
    backup=$(mktemp)
    if [ -f "$UPSTREAM_FILE" ]; then
        had_file=1
        cp "$UPSTREAM_FILE" "$backup"
    fi

    echo "server 127.0.0.1:$port;" | sudo tee "$UPSTREAM_FILE" > /dev/null

    # `nginx -s reload` exits 0 even when the config is rejected — the master
    # just keeps the old one — so validate first. And restore on *either*
    # failure: this file is the record of which slot is live, and the next
    # deploy trusts it over systemd, so leaving it naming a slot nginx never
    # picked up makes that deploy stop the slot still serving traffic.
    if ! sudo nginx -t || ! sudo nginx -s reload; then
        echo "ERROR: nginx would not switch to $colour; restoring the previous upstream." >&2
        if [ "$had_file" -eq 1 ]; then
            sudo tee "$UPSTREAM_FILE" < "$backup" > /dev/null
        else
            sudo rm -f "$UPSTREAM_FILE"
        fi
        rm -f "$backup"
        return 1
    fi

    rm -f "$backup"
    echo "→ nginx now routes to $colour (port $port)"
}
