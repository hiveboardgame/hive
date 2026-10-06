# shellcheck shell=bash
# shellcheck disable=SC2034

PROJECT_ROOT="/home/drone/hive"
BUILD_DIR="/home/drone/hive-build"
BIN_DIR="$PROJECT_ROOT/bin"
RELEASES_DIR="$PROJECT_ROOT/releases"
ACTIVE_FILE="$RELEASES_DIR/active"
UPSTREAM_FILE="/etc/nginx/hive-upstream.conf"
NGINX_LOCAL="http://127.0.0.1:3999"
PG_SERVICE="hive"
RELEASES_KEEP=5
DRAIN_SECONDS=1
HEALTH_TIMEOUT=60
LIVENESS_PROBE_TIMEOUT=3
READY_PROBE_TIMEOUT=15
DESTRUCTIVE_SQL='DROP[[:space:]]+(MATERIALIZED[[:space:]]+)?(COLUMN|TABLE|INDEX|CONSTRAINT|TYPE|SEQUENCE|VIEW|FUNCTION|TRIGGER|SCHEMA)[[:space:]]|ALTER[[:space:]]+TABLE[^;]*[[:space:]]DROP[[:space:]]|ALTER[[:space:]]+(TABLE|COLUMN|TYPE)[^;]*[[:space:]](RENAME|TYPE|SET[[:space:]]+NOT[[:space:]]+NULL)([[:space:];]|$)|ADD[[:space:]]+CONSTRAINT[[:space:]]|CREATE[[:space:]]+(OR[[:space:]]+REPLACE[[:space:]]+)?(CONSTRAINT[[:space:]]+)?TRIGGER[[:space:]]|ADD[[:space:]]+(PRIMARY[[:space:]]+KEY|FOREIGN[[:space:]]+KEY|EXCLUDE)|ALTER[[:space:]]+TYPE[^;]*[[:space:]]ADD[[:space:]]+VALUE[[:space:]]|TRUNCATE[[:space:]]|DELETE[[:space:]]+FROM[[:space:]]'
UPSTREAM_HELPER="/usr/local/sbin/hive-upstream"
SCRIPTS_DIR="$PROJECT_ROOT/scripts"

slot_port() {
    case "$1" in
        blue)  echo 3000 ;;
        green) echo 3001 ;;
        *)     echo "ERROR: unknown slot '$1'" >&2; return 1 ;;
    esac
}

other_slot() {
    case "$1" in
        blue)  echo green ;;
        green) echo blue ;;
        *)     echo "ERROR: unknown slot '$1'" >&2; return 1 ;;
    esac
}

release_dir() { echo "$RELEASES_DIR/$1"; }

release_complete() { [ -n "$1" ] && [ -f "$(release_dir "$1")/.complete" ]; }

slot_release() {
    local link="$BIN_DIR/$1"
    [ -L "$link" ] || return 0
    basename "$(readlink "$link")"
}

read_marker() {
    local f="$RELEASES_DIR/$1"
    [ -f "$f" ] && cat "$f"
    return 0
}

start_run_log() {
    local log_dir="$PROJECT_ROOT/deploy-logs"
    mkdir -p "$log_dir"
    RUN_LOG="$log_dir/$(date +%F-%H%M%S)-$1.log"
    # Ignores SIGHUP so a dropped SSH session still logs cleanup; drops the deploy lock fd.
    exec > >(trap '' HUP; exec 9>&-; while IFS= read -r line; do printf '%s %s\n' "$(date +%T)" "$line"; done | tee -a "$RUN_LOG") 2>&1
    echo "→ Logging to $RUN_LOG"
    echo "→ $1 of $(git -C "$PROJECT_ROOT" rev-parse --short=12 HEAD) by $(id -un) on $(hostname)"
}

require_drone() {
    if [ "$(id -un)" != drone ]; then
        echo "ERROR: run this as drone (sudo su - drone), not as $(id -un)." >&2
        exit 1
    fi
}

take_deploy_lock() {
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

read_active_slot() {
    [ -f "$UPSTREAM_FILE" ] || { echo none; return 0; }
    case "$(grep -oE '127\.0\.0\.1:(3000|3001)' "$UPSTREAM_FILE" | head -1)" in
        127.0.0.1:3000) echo blue ;;
        127.0.0.1:3001) echo green ;;
        *)              echo none ;;
    esac
}

print_status() {
    local colour
    echo "nginx upstream : $(read_active_slot)  ($(tr -d '\n' 2>/dev/null < "$UPSTREAM_FILE" || echo 'no upstream file'))"
    for colour in blue green; do
        printf '%-6s :%5s  %-8s  boot=%-8s  release=%-12s  /health=%s\n' \
            "$colour" "$(slot_port "$colour")" \
            "$(systemctl is-active "hive@$colour" 2>/dev/null || true)" \
            "$(systemctl is-enabled "hive@$colour" 2>/dev/null || true)" \
            "$(slot_release "$colour" | cut -c1-12)" \
            "$(curl -s -m 2 "http://127.0.0.1:$(slot_port "$colour")/health" 2>/dev/null | cut -c1-12 || true)"
    done
    echo "via nginx      : $(nginx_health)"
    echo "active (jobs+ws): $(active_instance_slot)"
    echo "current        : $(read_marker current | cut -c1-12)"
    echo "previous       : $(read_marker previous | cut -c1-12)"
}

# Echoes the live slot, or prints every inconsistency plus the status table and exits.
require_consistent_state() {
    local active idle current problems=()
    active=$(read_active_slot)
    if [ "$active" = none ]; then
        echo "ERROR: no upstream in $UPSTREAM_FILE. Before the first cutover: see 'First cutover' in scripts/README.md." >&2
        print_status >&2
        exit 1
    fi
    idle=$(other_slot "$active")
    current=$(read_marker current)

    systemctl is-active --quiet "hive@$active" || problems+=("hive@$active is live in nginx but not running")
    systemctl is-active --quiet "hive@$idle" && problems+=("hive@$idle is running but nginx does not route to it")
    [ "$(systemctl is-enabled "hive@$active" 2>/dev/null)" = enabled ] || problems+=("hive@$active is not enabled for boot")
    [ "$(systemctl is-enabled "hive@$idle" 2>/dev/null)" = enabled ] && problems+=("hive@$idle is enabled for boot but not live")
    [ -n "$current" ] || problems+=("no $RELEASES_DIR/current marker")
    [ "$(slot_release "$active")" = "$current" ] || problems+=("bin/$active does not point at the current release")
    release_complete "$current" || problems+=("release $current is missing or incomplete")
    [ "$(active_instance_slot)" = "$active" ] || problems+=("$ACTIVE_FILE does not name $active, so jobs and websockets are off there")

    if [ "${#problems[@]}" -gt 0 ]; then
        echo "ERROR: deployment state is inconsistent; refusing to guess:" >&2
        printf '    - %s\n' "${problems[@]}" >&2
        echo "" >&2
        print_status >&2
        echo "" >&2
        echo "See \"Recovering\" in scripts/README.md." >&2
        exit 1
    fi
    echo "$active"
}

# Judged per statement on the flattened SQL, so a statement split over lines still matches.
looks_destructive() {
    local sql stmt part column index table
    sql=$(sed -E 's/(^|[[:space:]])--.*$//' | tr '\n' ' ')
    grep -qiE "$DESTRUCTIVE_SQL" <<< "$sql" && return 0
    # An index on a table this migration creates is harmless; on an existing one a unique index
    # rejects the old release's writes and a plain build blocks them while it runs.
    while IFS= read -r index; do
        [ -n "$index" ] || continue
        table=$(grep -oiE '[[:space:]]ON[[:space:]]+(ONLY[[:space:]]+)?[A-Za-z0-9_."]+' <<< "$index" | head -1 | awk '{print $NF}')
        [ -n "$table" ] || return 0
        grep -oiE 'CREATE[[:space:]]+TABLE[[:space:]]+(IF[[:space:]]+NOT[[:space:]]+EXISTS[[:space:]]+)?[A-Za-z0-9_."]+' <<< "$sql" \
            | awk '{print $NF}' | grep -qixF "$table" && continue
        grep -qiE '^CREATE[[:space:]]+UNIQUE' <<< "$index" && return 0
        grep -qiE '[[:space:]]CONCURRENTLY[[:space:]]' <<< "$index" || return 0
    done < <(grep -oiE 'CREATE[[:space:]]+(UNIQUE[[:space:]]+)?INDEX[^;]*' <<< "$sql")
    while IFS= read -r -d ';' stmt; do
        column=""
        while IFS= read -r part; do
            if grep -qiE '(^|[[:space:]])ADD[[:space:]]' <<< "$part"; then
                added_column_is_destructive "$column" && return 0
                column="$part"
            elif [ -n "$column" ]; then
                column="$column,$part"
            fi
        done < <(tr ',' '\n' <<< "$stmt")
        added_column_is_destructive "$column" && return 0
    done <<< "$sql;"
    return 1
}

# The old release keeps inserting rows without this column, so it must not reject them.
added_column_is_destructive() {
    [ -n "$1" ] || return 1
    grep -qiE '[[:space:]](UNIQUE|CHECK)([[:space:]]|\(|$)' <<< "$1" && return 0
    grep -qiE 'NOT[[:space:]]+NULL' <<< "$1" && ! grep -qiE '[[:space:]]DEFAULT([[:space:]]|\()' <<< "$1"
}

migration_version() {
    local dir="${1#db/migrations/}"
    dir="${dir%%/*}"
    dir="${dir%%_*}"
    echo "${dir//-/}"
}

migration_versions_in() {
    local dir
    git ls-tree --name-only "$1:db/migrations" | while IFS= read -r dir; do
        migration_version "$dir"
    done
}

applied_migration_versions() {
    psql "service=$PG_SERVICE" -XAtqc 'SELECT version FROM __diesel_schema_migrations'
}

# Echoes "<commit> <path>" of an up.sql for the version. The given commit's own copy wins;
# otherwise the newest copy on any ref.
migration_source() {
    local version="$1" path commit
    shift
    if [ "$#" -gt 0 ]; then
        path=$(git ls-tree -r --name-only "$1" -- db/migrations/ \
            | grep -E '/up\.sql$' \
            | while IFS= read -r p; do
                if [ "$(migration_version "$p")" = "$version" ]; then echo "$p"; fi
            done | head -1 || true)
        if [ -n "$path" ]; then
            echo "$1 $path"
            return 0
        fi
    fi
    path=$(git log --all "$@" --format= --name-only --diff-filter=AM -- db/migrations/ \
        | grep -E '/up\.sql$' | sort -u \
        | while IFS= read -r p; do
            if [ "$(migration_version "$p")" = "$version" ]; then echo "$p"; fi
        done | head -1)
    [ -n "$path" ] || return 1
    commit=$(git log --all "$@" -1 --format=%H --diff-filter=AM -- "$path")
    [ -n "$commit" ] || return 1
    echo "$commit $path"
}

# Can `release` run against the schema in the database, plus `incoming`'s migrations if given?
# Every migration it does not know must look additive. 0 yes, 1 suspicious, 2 cannot tell.
check_schema_compat() {
    local release="$1" incoming="${2:-}" schema incoming_versions known unknown version source content destructive=()
    if ! schema=$(applied_migration_versions); then
        echo "ERROR: cannot read applied migrations through service=$PG_SERVICE." >&2
        return 2
    fi
    if [ -n "$incoming" ]; then
        if ! incoming_versions=$(migration_versions_in "$incoming"); then
            echo "ERROR: cannot list the migrations of ${incoming:0:12}." >&2
            return 2
        fi
        schema=$(printf '%s\n%s\n' "$schema" "$incoming_versions")
    fi
    if ! known=$(migration_versions_in "$release"); then
        echo "ERROR: cannot list the migrations of ${release:0:12}." >&2
        return 2
    fi
    unknown=$(comm -23 <(printf '%s\n' "$schema" | grep -v '^$' | LC_ALL=C sort -u) \
                       <(printf '%s\n' "$known" | LC_ALL=C sort -u))
    [ -n "$unknown" ] || return 0

    echo "→ Migrations release ${release:0:12} does not know:"
    while IFS= read -r version; do
        if ! source=$(migration_source "$version" ${incoming:+"$incoming"}); then
            echo "ERROR: migration $version is in the schema but not in git; cannot judge it." >&2
            return 2
        fi
        echo "    ${source#* }"
        if ! content=$(git show "${source%% *}:${source#* }"); then
            echo "ERROR: cannot read ${source#* }." >&2
            return 2
        fi
        if looks_destructive <<< "$content"; then
            destructive+=("${source#* }")
        fi
    done <<< "$unknown"

    if [ "${#destructive[@]}" -gt 0 ]; then
        echo "WARNING: release ${release:0:12} may not work against, or these lock a busy table:" >&2
        printf '    %s\n' "${destructive[@]}" >&2
        return 1
    fi
    return 0
}

active_instance_slot() {
    case "$(cat "$ACTIVE_FILE" 2>/dev/null)" in
        127.0.0.1:3000) echo blue ;;
        127.0.0.1:3001) echo green ;;
        *)              echo none ;;
    esac
}

set_active_instance() {
    mkdir -p "$RELEASES_DIR"
    echo "127.0.0.1:$(slot_port "$1")" > "$ACTIVE_FILE.tmp"
    mv "$ACTIVE_FILE.tmp" "$ACTIVE_FILE"
    echo "→ $1 is now the active instance (jobs and websockets)"
}

verify_slot() {
    local colour="$1" expected_sha="$2" port status body deadline binds addr
    port=$(slot_port "$colour")

    deadline=$(( $(date +%s) + HEALTH_TIMEOUT ))
    echo "→ Waiting for /health on :$port (up to ${HEALTH_TIMEOUT}s)..."
    while ! body=$(curl -sf -m "$LIVENESS_PROBE_TIMEOUT" "http://127.0.0.1:$port/health" 2>/dev/null); do
        if [ "$(date +%s)" -ge "$deadline" ]; then
            echo "ERROR: $colour failed liveness within ${HEALTH_TIMEOUT}s. Recent logs:" >&2
            sudo journalctl -u "hive@$colour" -n 50 --no-pager >&2 || true
            return 1
        fi
        sleep 1
    done
    if [ "$body" != "$expected_sha" ]; then
        echo "ERROR: $colour answers as release '$body', expected $expected_sha." >&2
        return 1
    fi
    echo "  liveness OK, release ${body:0:12}"

    binds=$(ss -tln "sport = :$port" | awk 'NR>1 {print $4}')
    if [ -z "$binds" ]; then
        echo "ERROR: $colour answered /health but nothing is listening on :$port" >&2
        return 1
    fi
    while read -r addr; do
        case "${addr%:*}" in
            127.0.0.1|"[::1]") ;;
            *)
                echo "ERROR: $colour is bound to $addr, not loopback." >&2
                echo "       Check LEPTOS_SITE_ADDR in /etc/hive/$colour.env." >&2
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
        return 1
    fi
    echo "  readiness OK ($body)"

    if ! curl -sf -m "$READY_PROBE_TIMEOUT" -o /dev/null "http://127.0.0.1:$port/"; then
        echo "ERROR: $colour / did not return 200" >&2
        sudo journalctl -u "hive@$colour" -n 50 --no-pager >&2 || true
        return 1
    fi
    echo "  SSR root OK"
}

# Echoes "<release sha> <slot address>" as seen through nginx.
nginx_health() {
    local response sha addr
    response=$(curl -s -i -m 3 "$NGINX_LOCAL/health" 2>/dev/null | tr -d '\r') || true
    sha=$(printf '%s\n' "$response" | tail -1)
    addr=$(printf '%s\n' "$response" | grep -i '^x-hive-addr:' | awk '{print $2}' | head -1)
    echo "$sha $addr"
}

verify_via_nginx() {
    local expected="$1 127.0.0.1:$(slot_port "$2")" seen attempt
    for attempt in 1 2 3 4 5; do
        seen=$(nginx_health)
        if [ "$seen" = "$expected" ]; then
            echo "  nginx serves release ${1:0:12} from $2"
            return 0
        fi
        sleep 1
    done
    echo "ERROR: through nginx the app answers as '$seen', expected '$expected'." >&2
    return 1
}

swap_upstream() {
    local colour="$1"
    if ! sudo "$UPSTREAM_HELPER" "$colour"; then
        echo "ERROR: nginx would not switch to $colour; the previous upstream is still in place." >&2
        return 1
    fi
    echo "→ nginx now routes to $colour (port $(slot_port "$colour"))"
}

# Only before the first deploy: the hand-started process on :3000 that predates the slots.
stop_old_process() {
    local waited=0
    [ -n "$(ss -tlnH "sport = :3000")" ] || return 0
    OLD_PID=$(ss -tlnpH "sport = :3000" | grep -oE 'pid=[0-9]+' | head -1 | cut -d= -f2 || true)
    if [ -z "$OLD_PID" ]; then
        echo "ERROR: something listens on :3000 but is not drone's; stop it by hand." >&2
        return 1
    fi
    echo "→ Stopping the old process on :3000 (pid $OLD_PID)..."
    kill -TERM "$OLD_PID"
    # Waiting on the pid, not the port: actix closes the listener at once but would keep
    # serving websockets and running jobs for its 30s drain, which protects nothing here.
    while kill -0 "$OLD_PID" 2>/dev/null; do
        if [ "$waited" -eq 3 ]; then
            echo "  sending SIGKILL instead of waiting out its 30s drain" >&2
            kill -KILL "$OLD_PID" 2>/dev/null || true
        fi
        sleep 1
        waited=$((waited + 1))
    done
    echo "  old process exited"
}

# The side that served before the flip: a slot, or "none" for the old process on :3000.
old_side_gone() {
    if [ "$1" = none ]; then
        [ -z "$(ss -tlnH "sport = :3000")" ] || return 1
        [ -z "${OLD_PID:-}" ] || ! kill -0 "$OLD_PID" 2>/dev/null
        return
    fi
    case "$(systemctl is-active "hive@$1" 2>/dev/null)" in
        inactive|failed) return 0 ;;
        *)               return 1 ;;
    esac
}

# Which side nginx actually serves, asked through nginx (reloads apply asynchronously, so the
# upstream file on disk can disagree). Only positive identification counts: blue or green by
# their X-Hive-Addr on a 200, old-process by an app answer without that header plus a rendered
# home page. Anything else, nginx's own 5xx pages included, is unknown.
nginx_route() {
    local response status addr page
    response=$(curl -s -i -m 3 "$NGINX_LOCAL/health" 2>/dev/null | tr -d '\r') || true
    status=$(printf '%s\n' "$response" | head -1 | awk '{print $2}')
    addr=$(printf '%s\n' "$response" | grep -i '^x-hive-addr:' | awk '{print $2}' | head -1)
    if [ "$status" = 200 ] && [ -n "$addr" ]; then
        case "$addr" in
            127.0.0.1:3000) echo blue; return ;;
            127.0.0.1:3001) echo green; return ;;
        esac
    fi
    if [ -z "$addr" ] && { [ "$status" = 200 ] || [ "$status" = 404 ]; }; then
        page=$(curl -s -m 5 -w '\n%{http_code}' "$NGINX_LOCAL/" 2>/dev/null) || true
        if [ "$(printf '%s\n' "$page" | tail -1)" = 200 ] && grep -q '<title>HiveGame.com' <<< "$page"; then
            echo old-process
            return
        fi
    fi
    echo unknown
}

# Polls until nginx positively serves $1; up to 10s.
wait_until_nginx_serves() {
    local tries=0
    while [ "$tries" -lt 20 ]; do
        [ "$(nginx_route)" = "$1" ] && return 0
        sleep 0.5
        tries=$((tries + 1))
    done
    return 1
}

# After a failure or interrupt: make nginx, the slots and releases/active agree again, judged
# by what nginx actually serves. $1 the new slot, $2 the old side (a slot, or "none" for the
# old process on :3000), $3 1 once the new slot was verified through nginx. The caller defines
# write_live_markers. Never stops the new slot while nginx may still route to it.
recover_after_failure() {
    local new="$1" old="$2" verified="$3" old_route="$2" old_seen="$2" route waited=0
    if [ "$old" = none ]; then
        old_route=blue
        old_seen=old-process
    fi
    if [ "$(active_instance_slot)" = "$new" ]; then
        write_live_markers
        return 0
    fi
    # Never route back to a side that is already gone.
    if old_side_gone "$old"; then
        if [ "$(read_active_slot)" != "$new" ]; then
            swap_upstream "$new" >&2 || true
        fi
        if [ "$verified" = 1 ]; then
            echo "→ The old side is gone; finishing the handoff to $new." >&2
            if [ "$old" = none ]; then set_boot_slot "$new" ""; else set_boot_slot "$new" "$old"; fi
            set_active_instance "$new" >&2
            write_live_markers
            return 0
        fi
        echo "ERROR: the old side is gone and $new was never verified; nginx routes to $new." >&2
        echo "       Check it (scripts/smoke.sh), then see \"Recovering\" in scripts/README.md." >&2
        return 1
    fi
    route=$(nginx_route)
    if [ "$verified" = 1 ] && [ "$route" = "$new" ]; then
        echo "→ nginx serves the verified $new; finishing the handoff." >&2
        if [ "$old" = none ]; then
            stop_old_process >&2
        else
            sudo systemctl stop "hive@$old"
        fi
        while ! old_side_gone "$old"; do
            if [ "$waited" -ge 60 ]; then
                echo "ERROR: the old side is still running after 60s. Once it has stopped, run:" >&2
                echo "    echo 127.0.0.1:$(slot_port "$new") > $ACTIVE_FILE" >&2
                echo "and fix the markers as in \"Recovering\" (scripts/README.md)." >&2
                return 1
            fi
            sleep 1
            waited=$((waited + 1))
        done
        if [ "$old" = none ]; then set_boot_slot "$new" ""; else set_boot_slot "$new" "$old"; fi
        set_active_instance "$new" >&2
        write_live_markers
        return 0
    fi
    if [ "$route" != "$old_seen" ] || [ "$(read_active_slot)" != "$old_route" ]; then
        echo "→ Routing nginx back to $old_route before stopping $new." >&2
        swap_upstream "$old_route" >&2 || true
    fi
    if ! wait_until_nginx_serves "$old_seen"; then
        echo "ERROR: cannot confirm that nginx serves $old_seen again; leaving $new running. Run:" >&2
        echo "    sudo $UPSTREAM_HELPER $old_route   # until the old side answers through nginx" >&2
        echo "    sudo systemctl stop hive@$new" >&2
        return 1
    fi
    echo "→ nginx serves the old side; stopping $new." >&2
    sudo systemctl stop "hive@$new"
}

# Ends a deploy or rollback with one verdict line either way.
final_verdict() {
    local slot="$1" sha="$2"
    echo
    if "$SCRIPTS_DIR/smoke.sh"; then
        echo "OK: $slot is live with release ${sha:0:12}; smoke passed."
        return 0
    fi
    echo "DEPLOYED, BUT SMOKE FAILED: $slot is live with release ${sha:0:12}. See the FAIL lines above;"
    echo "scripts/rollback.sh goes back to the previous release."
    return 1
}

set_boot_slot() {
    local enable="$1" disable="$2"
    sudo systemctl enable "hive@$enable" || return 1
    if [ -n "$disable" ]; then
        sudo systemctl disable "hive@$disable" || return 1
    fi
}

point_slot() {
    local colour="$1" sha="$2"
    mkdir -p "$BIN_DIR"
    if [ -e "$BIN_DIR/$colour" ] && [ ! -L "$BIN_DIR/$colour" ]; then
        echo "ERROR: $BIN_DIR/$colour exists and is not a symlink; move it away first." >&2
        return 1
    fi
    ln -sfn "../releases/$sha" "$BIN_DIR/$colour"
}

record_live_release() {
    local sha="$1" previous
    previous=$(read_marker current)
    if [ -n "$previous" ] && [ "$previous" != "$sha" ]; then
        echo "$previous" > "$RELEASES_DIR/previous"
    fi
    echo "$sha" > "$RELEASES_DIR/current"
}

prune_releases() {
    local keep=() dir sha kept=0
    keep+=("$(read_marker current)" "$(read_marker previous)" "$(slot_release blue)" "$(slot_release green)")
    while IFS= read -r dir; do
        sha=$(basename "$dir")
        if printf '%s\n' "${keep[@]}" | grep -qxF "$sha"; then
            continue
        fi
        kept=$((kept + 1))
        [ "$kept" -le "$RELEASES_KEEP" ] && continue
        rm -rf "$dir"
    done < <(find "$RELEASES_DIR" -mindepth 1 -maxdepth 1 -type d ! -name '*.new' -printf '%T@ %p\n' | sort -rn | cut -d' ' -f2-)
}
