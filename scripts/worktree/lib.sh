# shellcheck shell=bash
# shellcheck disable=SC2034

WT_LIB=${WT_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)}

main_checkout_or_empty() {
    local common
    common=$(git rev-parse --git-common-dir 2> /dev/null) || return 0
    (cd "$common/.." && pwd -P)
}

default_proxy_port() {
    local start
    start=$(cat /proc/sys/net/ipv4/ip_unprivileged_port_start 2> /dev/null || echo 0)
    if [ "$start" -gt 80 ]; then echo 8080; else echo 80; fi
}

MAIN_DIR=$(main_checkout_or_empty)
WT_ROOT="${HIVE_WORKTREES_DIR:-${MAIN_DIR:+$(dirname "$MAIN_DIR")/$(basename "$MAIN_DIR")-worktrees}}"
PROXY_PORT="${HIVE_PROXY_PORT:-$(default_proxy_port)}"
CADDYFILE="$WT_ROOT/Caddyfile"
CADDY_DIR="$WT_ROOT/caddy"
CADDY_ADMIN="http://localhost:2019"
GITHUB_REPO=hiveboardgame/hive
ENV_FILE=".worktree.env"
PORT_BASE=3100
PORT_STEP=10
PG_HOST=localhost
PG_USER=hive-dev
TEMPLATE_DB=hive-template
WORKTREE_RUNTIME=${HIVE_WORKTREE_RUNTIME:-native}
MAIN_NAME=hive
MAIN_PORT=3000
MAIN_DB=hive-local
MAIN_TEST_DB=hive-test
PROTECTED_DBS=(postgres "$MAIN_DB" "$MAIN_TEST_DB" "$TEMPLATE_DB")
CLAUDE_PROJECTS="$HOME/.claude/projects"
LOCAL_FILES=(AGENTS.md CLAUDE.md .agents docs hivegame-games.csv.gz .claude/settings.local.json)
HEALTH_TIMEOUT_SECS=900
NETWORK_TIMEOUT_SECS=20
SHARE_IPS=()
SHARE_LABELS=()

log() { printf '%s\n' "$*" >&2; }
warn() { log "warning: $*"; }
die() {
    log "error: $*"
    exit 1
}

require_commands() {
    local command
    for command in "$@"; do
        command -v "$command" > /dev/null 2>&1 || die "$command is required but was not found on PATH; install it, then rerun this command"
    done
}

require_runtime() {
    case $WORKTREE_RUNTIME in
        native) ;;
        nix) require_commands nix ;;
        *) die "HIVE_WORKTREE_RUNTIME must be native or nix" ;;
    esac
}

validate_template_name() {
    [[ $1 =~ ^hive-template(-[a-z0-9]+)*$ ]] && [ "${#1}" -le 63 ] ||
        die "template name must be hive-template or hive-template-<name> (at most 63 characters)"
}

confirm() {
    local reply
    read -r -p "$1 [y/N] " reply
    [[ $reply == [yY] ]]
}

require_repo() {
    require_commands git
    [ -n "$MAIN_DIR" ] || die "run this inside the hive checkout or one of its worktrees"
    [ -n "$WT_ROOT" ] || die "could not work out where worktrees live; set HIVE_WORKTREES_DIR"
}

main_checkout() {
    require_repo
    echo "$MAIN_DIR"
}

require_createdb() {
    [ "$(psql_admin -tAc 'select rolcreatedb or rolsuper from pg_roles where rolname = current_user')" = t ] && return
    die "role $PG_USER may not create databases; as a Postgres superuser run:
  psql -h $PG_HOST -d postgres -c 'ALTER ROLE \"$PG_USER\" CREATEDB'"
}

sanitize_name() {
    printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9-]+/-/g; s/^-+//; s/-+$//'
}

wt_dir() { echo "$WT_ROOT/$1"; }
env_path() { echo "$(wt_dir "$1")/$ENV_FILE"; }
db_name() { echo "hive-$1"; }
test_db_name() { echo "hive-$1-test"; }
db_url() { echo "postgres://$PG_USER@$PG_HOST/$1"; }
proxy_host() {
    if [ "$PROXY_PORT" = 80 ]; then echo "$1.localhost"; else echo "$1.localhost:$PROXY_PORT"; fi
}
origin_for() { echo "http://$(proxy_host "$1")"; }

valid_ipv4() {
    local address=$1 octet
    [[ $address =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] || return 1
    for octet in ${address//./ }; do
        [ "$((10#$octet))" -le 255 ] || return 1
    done
    [[ $address != 0.* && $address != 127.* ]]
}

lan_addresses() {
    local route interface
    if command -v ip > /dev/null; then
        while IFS= read -r route; do
            if [[ $route =~ [[:space:]]src[[:space:]]([0-9.]+) ]]; then
                echo "${BASH_REMATCH[1]}"
            elif [[ $route =~ [[:space:]]dev[[:space:]]([^[:space:]]+) ]]; then
                interface=${BASH_REMATCH[1]}
                ip -o -4 addr show dev "$interface" scope global | sed -nE 's/.* inet ([0-9.]+)\/.*/\1/p'
            fi
        done < <(ip -o -4 route show default)
    elif command -v ipconfig > /dev/null && command -v route > /dev/null; then
        interface=$(route -n get default 2> /dev/null | sed -nE 's/^[[:space:]]*interface: (.*)/\1/p')
        [ -z "$interface" ] || ipconfig getifaddr "$interface"
    fi
}

detect_share_addresses() {
    local lan_override=$1 tailnet_override=$2 address existing seen candidates tailnet_candidates label
    candidates=$lan_override
    if [ -z "$candidates" ]; then candidates=$(lan_addresses); fi
    tailnet_candidates=$tailnet_override
    if [ -z "$tailnet_candidates" ] && command -v tailscale > /dev/null; then
        tailnet_candidates=$(tailscale ip -4 2> /dev/null || true)
    fi
    candidates+=" $tailnet_candidates"
    SHARE_IPS=()
    SHARE_LABELS=()
    for address in $candidates; do
        valid_ipv4 "$address" || die "invalid share IPv4 address: $address"
        label=LAN
        for existing in $tailnet_candidates; do
            [ "$existing" != "$address" ] || label=Tailnet
        done
        seen=0
        for existing in "${SHARE_IPS[@]}"; do
            [ "$existing" != "$address" ] || seen=1
        done
        if [ "$seen" = 0 ]; then
            SHARE_IPS+=("$address")
            SHARE_LABELS+=("$label")
        fi
    done
    [ "${#SHARE_IPS[@]}" -gt 0 ] || die "no LAN or Tailnet IPv4 address found; pass --lan-ip or --tailnet-ip with --share"
}

shared_origin_for() {
    local name=$1 address=$2 host
    valid_ipv4 "$address" || die "invalid saved share IPv4 address: $address"
    host="$name.${address//./-}.sslip.io"
    [ "$PROXY_PORT" = 80 ] || host+=":$PROXY_PORT"
    echo "http://$host"
}

save_share_addresses() {
    local name=$1
    if [ "${#SHARE_IPS[@]}" -gt 0 ]; then
        mkdir -p "$CADDY_DIR/share"
        printf '%s\n' "${SHARE_IPS[@]}" > "$CADDY_DIR/share/$name"
    else
        rm -f "$CADDY_DIR/share/$name"
    fi
}

load_share_addresses() {
    local name=$1 address
    SHARE_IPS=()
    [ -f "$CADDY_DIR/share/$name" ] || return 0
    while IFS= read -r address; do
        valid_ipv4 "$address" || die "invalid saved share IPv4 address: $address"
        SHARE_IPS+=("$address")
    done < "$CADDY_DIR/share/$name"
}

instance_origins() {
    local name=$1 address
    origin_for "$name"
    if [ "$name" = "$MAIN_NAME" ]; then
        printf 'http://localhost:%s\nhttp://127.0.0.1:%s\n' "$PROXY_PORT" "$PROXY_PORT"
    fi
    if [ -f "$CADDY_DIR/share/$name" ]; then
        while IFS= read -r address; do
            shared_origin_for "$name" "$address"
        done < "$CADDY_DIR/share/$name"
    fi
}

instances() {
    local d
    for d in "$WT_ROOT"/*/; do
        [ -f "$d$ENV_FILE" ] && basename "$d"
    done
    return 0
}

load_env() {
    local name=$1
    # Share private app settings, then let each instance select its own database and outputs.
    if [ -f "$MAIN_DIR/$ENV_FILE" ]; then
        set -a
        # shellcheck source=/dev/null
        source "$MAIN_DIR/$ENV_FILE"
        set +a
    fi
    WT_SEED_TEMPLATE=$TEMPLATE_DB
    if [ "$name" = "$MAIN_NAME" ]; then
        WT_NAME=$MAIN_NAME
        WT_DIR=$(main_checkout)
        WT_BRANCH=$(git -C "$WT_DIR" branch --show-current)
        WT_PORT=$MAIN_PORT
        WT_DB=$MAIN_DB
        WT_TEST_DB=$MAIN_TEST_DB
        return
    fi
    local file
    file=$(env_path "$name")
    [ -f "$file" ] || die "no instance named '$name' (expected $file); see: hive-setup --ls"
    # shellcheck source=/dev/null
    source "$file"
    validate_template_name "$WT_SEED_TEMPLATE"
    WT_DIR=$(wt_dir "$name")
    WT_DB=$(db_name "$name")
    WT_TEST_DB=$(test_db_name "$name")
}

write_env() {
    local name=$1 branch=$2 port=$3 seed=${4:-${WT_SEED_TEMPLATE:-$TEMPLATE_DB}} origin dir
    origin=$(origin_for "$name")
    dir=$(wt_dir "$name")
    # This file is also sourced by run/direnv; quote branch names and paths as shell data.
    (
        umask 077
        {
            printf 'WT_NAME=%q\nWT_BRANCH=%q\nWT_PORT=%q\nWT_SEED_TEMPLATE=%q\n' "$name" "$branch" "$port" "$seed"
            printf 'CARGO_TARGET_DIR=%q\nLEPTOS_SITE_ROOT=%q\n' "$dir/.cargo/target" "$dir/target/site"
            cat << EOT
DATABASE_URL=$(db_url "$(db_name "$name")")
TEST_DATABASE_URL=$(db_url "$(test_db_name "$name")")
LEPTOS_SITE_ADDR=127.0.0.1:$port
LEPTOS_RELOAD_PORT=$((port + 1))
APP_ORIGIN=$origin
EMAIL_BASE_URL=$origin
WS_TELEMETRY_INTERVAL_SECS=0
EOT
        } > "$(env_path "$name")"
    )
}

export_instance_env() {
    local name=$1 port=$2 db=$3 test_db=$4 origin address additional=""
    origin=$(origin_for "$name")
    if [ "$name" != "$MAIN_NAME" ]; then
        export DATABASE_URL TEST_DATABASE_URL LEPTOS_SITE_ADDR EMAIL_BASE_URL
        DATABASE_URL=$(db_url "$db")
        TEST_DATABASE_URL=$(db_url "$test_db")
        LEPTOS_SITE_ADDR="127.0.0.1:$port"
        EMAIL_BASE_URL=$origin
    else
        additional="http://localhost:$port,http://127.0.0.1:$port,http://localhost:$PROXY_PORT,http://127.0.0.1:$PROXY_PORT"
    fi
    for address in "${SHARE_IPS[@]}"; do
        additional+="${additional:+,}$(shared_origin_for "$name" "$address")"
    done
    unset APP_DEV_ORIGINS
    if [ -n "$additional" ]; then export APP_DEV_ORIGINS=$additional; fi
    export LEPTOS_RELOAD_PORT LEPTOS_RELOAD_EXTERNAL_PORT LEPTOS_RELOAD_WS_PROTOCOL APP_ORIGIN WS_TELEMETRY_INTERVAL_SECS HIVE_WORKTREE HIVE_WORKTREE_ROOT CARGO_TARGET_DIR LEPTOS_SITE_ROOT
    LEPTOS_RELOAD_PORT=$((port + 1))
    LEPTOS_RELOAD_EXTERNAL_PORT=$PROXY_PORT
    LEPTOS_RELOAD_WS_PROTOCOL=WS
    APP_ORIGIN=$origin
    WS_TELEMETRY_INTERVAL_SECS=0
    HIVE_WORKTREE=$name
    HIVE_WORKTREE_ROOT=$WT_DIR
    CARGO_TARGET_DIR="$WT_DIR/.cargo/target"
    LEPTOS_SITE_ROOT="$WT_DIR/target/site"
}

psql_admin() { psql -X -q -v ON_ERROR_STOP=1 -h "$PG_HOST" -U "$PG_USER" -d postgres "$@"; }
psql_db() { psql -X -q -v ON_ERROR_STOP=1 -h "$PG_HOST" -U "$PG_USER" -d "$@"; }

db_exists() {
    [ "$(psql_admin -tAc "select 1 from pg_database where datname = '$1'")" = 1 ]
}

db_size() { psql_admin -tAc "select pg_size_pretty(pg_database_size('$1'))"; }

db_size_bytes() {
    psql_admin -At -v database="$1" << 'SQL'
SELECT pg_database_size(:'database');
SQL
}

worktree_dbs() {
    managed_dbs | while IFS='|' read -r db identity; do
        is_protected_db "$db" || printf '%s|%s\n' "$db" "$identity"
    done
}

managed_dbs() {
    psql_admin -At -v owner="hive-worktree:$MAIN_DIR" << 'SQL'
SELECT datname, substring(marker FROM length(:'owner') + 2)
FROM (
    SELECT datname, shobj_description(oid, 'pg_database') AS marker
    FROM pg_database
) AS databases
WHERE marker = :'owner' OR starts_with(marker, :'owner' || ':')
ORDER BY datname;
SQL
}

unowned_dbs() {
    psql_admin -At << 'SQL'
SELECT datname
FROM pg_database
WHERE starts_with(datname, 'hive-')
    AND shobj_description(oid, 'pg_database') IS NULL
    AND pg_get_userbyid(datdba) = current_user
ORDER BY datname;
SQL
}

mark_worktree_db() {
    psql_admin -v database="$1" -v owner="hive-worktree:$MAIN_DIR:$2:$3" << 'SQL'
COMMENT ON DATABASE :"database" IS :'owner';
SQL
}

is_protected_db() {
    local p
    # Reserve the whole template namespace, including temporarily unused seeds.
    [[ $1 == hive-template || $1 == hive-template-* ]] && return 0
    for p in "${PROTECTED_DBS[@]}"; do
        [ "$1" = "$p" ] && return 0
    done
    return 1
}

terminate_connections() {
    psql_admin -tAc "select pg_terminate_backend(pid) from pg_stat_activity where datname = '$1' and pid <> pg_backend_pid()" > /dev/null
}

drop_db() {
    is_protected_db "$1" && die "refusing to drop protected database $1"
    terminate_connections "$1"
    dropdb --if-exists --force -h "$PG_HOST" -U "$PG_USER" "$1"
}

create_db() { createdb -h "$PG_HOST" -U "$PG_USER" -O "$PG_USER" "$@"; }

port_listening() { lsof -nP -iTCP:"$1" -sTCP:LISTEN -t > /dev/null 2>&1; }
listener_pid() { lsof -nP -iTCP:"$1" -sTCP:LISTEN -t 2> /dev/null | head -1; }
listener_cwd() { lsof -a -p "$1" -d cwd -Fn 2> /dev/null | sed -n 's/^n//p' | head -1; }

port_claimed() {
    grep -qs -x "WT_PORT=$1" "$WT_ROOT"/*/"$ENV_FILE" 2> /dev/null
}

alloc_port() {
    local p=$PORT_BASE
    while port_claimed "$p" || port_listening "$p" || port_listening "$((p + 1))"; do
        p=$((p + PORT_STEP))
    done
    echo "$p"
}

listener_is_instance() {
    local pid=$1 dir=$2
    [ "$(listener_cwd "$pid")" = "$dir" ] || [[ $(ps -o command= -p "$pid" 2> /dev/null) == *"$dir/"* ]]
}

instance_status() {
    local name=$1 port=$2 pid dir
    if ! port_listening "$port"; then
        echo "-"
        return
    fi
    pid=$(listener_pid "$port")
    dir=$(wt_dir "$name")
    [ "$name" = "$MAIN_NAME" ] && dir=$(main_checkout)
    if listener_is_instance "$pid" "$dir"; then
        echo "running (pid $pid)"
    else
        echo "port taken by pid $pid"
    fi
}

caddy_env() {
    export XDG_DATA_HOME="$CADDY_DIR/data" XDG_CONFIG_HOME="$CADDY_DIR/config"
}

caddy_regen() (
    local name tmp
    mkdir -p "$CADDY_DIR"
    tmp=$(mktemp)
    {
        printf '{\n\tadmin %s\n\tlog {\n\t\toutput file %s/caddy.log\n\t}\n}\n\n' "${CADDY_ADMIN#http://}" "$CADDY_DIR"
        caddy_instance "$MAIN_NAME" "$MAIN_PORT"
        for name in $(instances); do
            # shellcheck source=/dev/null
            source "$(env_path "$name")"
            caddy_instance "$name" "$WT_PORT"
        done
    } | caddy fmt - > "$tmp"
    mv "$tmp" "$CADDYFILE"
)

caddy_instance() {
    local name=$1 port=$2 origin
    while IFS= read -r origin; do
        printf '%s {\n' "$origin"
        printf '\t@reload path /live_reload\n\treverse_proxy @reload 127.0.0.1:%s\n' "$((port + 1))"
        printf '\treverse_proxy 127.0.0.1:%s\n}\n\n' "$port"
    done < <(instance_origins "$name")
}

caddy_running() { curl -sf -o /dev/null "$CADDY_ADMIN/config/"; }

caddy_ensure() {
    caddy_regen
    caddy_env
    if caddy_running; then
        caddy reload --config "$CADDYFILE" --adapter caddyfile
    else
        # Give the shared proxy its own process group so a watcher's Ctrl-C cannot stop it.
        (
            set -m
            caddy start --config "$CADDYFILE" --adapter caddyfile --pidfile "$CADDY_DIR/pid" < /dev/null &
            wait "$!"
        )
    fi
}

caddy_reload_if_running() {
    caddy_regen
    caddy_env
    if caddy_running; then
        caddy reload --config "$CADDYFILE" --adapter caddyfile
    fi
}

caddy_routes() {
    [ -f "$CADDYFILE" ] || return 0
    sed -nE 's/^http:\/\/([a-z0-9-]+)\.localhost(:[0-9]+)? \{$/\1/p' "$CADDYFILE" | grep -vx "$MAIN_NAME" || true
}

curl_instance() {
    local name=$1 path=$2
    curl -sf --max-time 3 --resolve "$name.localhost:$PROXY_PORT:127.0.0.1" "$(origin_for "$name")$path"
}

uses_claude() { [ -d "$CLAUDE_PROJECTS" ]; }
claude_project_dir() { echo "$CLAUDE_PROJECTS/$(printf '%s' "$1" | sed 's/[^A-Za-z0-9-]/-/g')"; }
main_memory_dir() { echo "$(claude_project_dir "$(main_checkout)")/memory"; }

worktree_registered() {
    git -C "$(main_checkout)" worktree list --porcelain | grep -qx "worktree $1"
}

worktree_dirty_count() {
    local dir=$1 pattern
    pattern=$(printf '%s|' "${LOCAL_FILES[@]}" | sed 's/|$//; s/\./\\./g')
    git -C "$dir" status --porcelain 2> /dev/null | grep -cvE "^\?\? ($pattern)$" || true
}
