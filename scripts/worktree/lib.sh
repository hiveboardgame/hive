# shellcheck shell=bash
# shellcheck disable=SC2034

main_checkout_or_empty() {
    local common
    common=$(git rev-parse --git-common-dir 2>/dev/null) || return 0
    (cd "$common/.." && pwd -P)
}

default_proxy_port() {
    local start
    start=$(cat /proc/sys/net/ipv4/ip_unprivileged_port_start 2>/dev/null || echo 0)
    if [ "$start" -gt 80 ]; then echo 8080; else echo 80; fi
}

MAIN_DIR=$(main_checkout_or_empty)
WT_ROOT="${HIVE_WORKTREES_DIR:-${MAIN_DIR:+$(dirname "$MAIN_DIR")/$(basename "$MAIN_DIR")-worktrees}}"
PROXY_PORT="${HIVE_PROXY_PORT:-$(default_proxy_port)}"
CADDYFILE="$WT_ROOT/Caddyfile"
CADDY_DIR="$WT_ROOT/caddy"
CADDY_ADMIN="http://localhost:2019"
ENV_FILE=".worktree.env"
PORT_BASE=3100
PORT_STEP=10
PG_HOST=localhost
PG_USER=hive-dev
TEMPLATE_DB=hive-template
MAIN_NAME=hive
MAIN_PORT=3000
MAIN_DB=hive-local
MAIN_TEST_DB=hive-test
PROTECTED_DBS=(postgres "$MAIN_DB" "$MAIN_TEST_DB" "$TEMPLATE_DB")
CLAUDE_PROJECTS="$HOME/.claude/projects"
LINKED_FILES=(CLAUDE.md docs hivegame-games.csv.gz .claude/settings.local.json)
HEALTH_TIMEOUT_SECS=900

log() { printf '%s\n' "$*" >&2; }
warn() { log "warning: $*"; }
die() { log "error: $*"; exit 1; }

confirm() {
    local reply
    read -r -p "$1 [y/N] " reply
    [[ $reply == [yY] ]]
}

require_repo() {
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

instances() {
    local d
    for d in "$WT_ROOT"/*/; do
        [ -f "$d$ENV_FILE" ] && basename "$d"
    done
    return 0
}

load_env() {
    local name=$1
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
    [ -f "$file" ] || die "no instance named '$name' (expected $file); see: setup --ls"
    # shellcheck source=/dev/null
    source "$file"
    WT_DIR=$(wt_dir "$name")
    WT_DB=$(db_name "$name")
    WT_TEST_DB=$(test_db_name "$name")
}

write_env() {
    local name=$1 branch=$2 port=$3 origin
    origin=$(origin_for "$name")
    cat > "$(env_path "$name")" <<EOT
WT_NAME=$name
WT_BRANCH=$branch
WT_PORT=$port
DATABASE_URL=$(db_url "$(db_name "$name")")
TEST_DATABASE_URL=$(db_url "$(test_db_name "$name")")
LEPTOS_SITE_ADDR=127.0.0.1:$port
LEPTOS_RELOAD_PORT=$((port + 1))
APP_ORIGIN=$origin
EMAIL_BASE_URL=$origin
WS_TELEMETRY_INTERVAL_SECS=0
EOT
}

export_instance_env() {
    local name=$1 port=$2 db=$3 test_db=$4 origin
    origin=$(origin_for "$name")
    export DATABASE_URL TEST_DATABASE_URL LEPTOS_SITE_ADDR LEPTOS_RELOAD_PORT APP_ORIGIN EMAIL_BASE_URL WS_TELEMETRY_INTERVAL_SECS HIVE_WORKTREE
    DATABASE_URL=$(db_url "$db")
    TEST_DATABASE_URL=$(db_url "$test_db")
    LEPTOS_SITE_ADDR="127.0.0.1:$port"
    LEPTOS_RELOAD_PORT=$((port + 1))
    APP_ORIGIN=$origin
    EMAIL_BASE_URL=$origin
    WS_TELEMETRY_INTERVAL_SECS=0
    HIVE_WORKTREE=$name
}

psql_admin() { psql -X -q -v ON_ERROR_STOP=1 -h "$PG_HOST" -U "$PG_USER" -d postgres "$@"; }
psql_db() { psql -X -q -v ON_ERROR_STOP=1 -h "$PG_HOST" -U "$PG_USER" -d "$@"; }

db_exists() {
    [ "$(psql_admin -tAc "select 1 from pg_database where datname = '$1'")" = 1 ]
}

db_size() { psql_admin -tAc "select pg_size_pretty(pg_database_size('$1'))"; }

worktree_dbs() {
    psql_admin -tAc "select datname from pg_database where datname like 'hive-%' order by 1" | while read -r db; do
        is_protected_db "$db" || echo "$db"
    done
}

is_protected_db() {
    local p
    for p in "${PROTECTED_DBS[@]}"; do
        [ "$1" = "$p" ] && return 0
    done
    return 1
}

terminate_connections() {
    psql_admin -tAc "select pg_terminate_backend(pid) from pg_stat_activity where datname = '$1' and pid <> pg_backend_pid()" >/dev/null
}

drop_db() {
    is_protected_db "$1" && die "refusing to drop protected database $1"
    terminate_connections "$1"
    dropdb --if-exists --force -h "$PG_HOST" -U "$PG_USER" "$1"
}

create_db() { createdb -h "$PG_HOST" -U "$PG_USER" -O "$PG_USER" "$@"; }

port_listening() { lsof -nP -iTCP:"$1" -sTCP:LISTEN -t >/dev/null 2>&1; }
listener_pid() { lsof -nP -iTCP:"$1" -sTCP:LISTEN -t 2>/dev/null | head -1; }
listener_cwd() { lsof -a -p "$1" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -1; }

port_claimed() {
    grep -qs -x "WT_PORT=$1" "$WT_ROOT"/*/"$ENV_FILE" 2>/dev/null
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
    [ "$(listener_cwd "$pid")" = "$dir" ] || [[ $(ps -o command= -p "$pid" 2>/dev/null) == *"$dir/"* ]]
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

caddy_regen() {
    local name tmp
    mkdir -p "$CADDY_DIR"
    tmp=$(mktemp)
    {
        printf '{\n\tadmin localhost:2019\n\tlog {\n\t\toutput file %s/caddy.log\n\t}\n}\n\n' "$CADDY_DIR"
        printf '%s {\n\treverse_proxy 127.0.0.1:%s\n}\n\n' "$(origin_for "$MAIN_NAME")" "$MAIN_PORT"
        for name in $(instances); do
            # shellcheck source=/dev/null
            source "$(env_path "$name")"
            printf '%s {\n\treverse_proxy 127.0.0.1:%s\n}\n\n' "$(origin_for "$name")" "$WT_PORT"
        done
    } > "$tmp"
    mv "$tmp" "$CADDYFILE"
}

caddy_running() { curl -sf -o /dev/null "$CADDY_ADMIN/config/"; }

caddy_ensure() {
    caddy_regen
    caddy_env
    if caddy_running; then
        caddy reload --config "$CADDYFILE" --adapter caddyfile
    else
        caddy start --config "$CADDYFILE" --adapter caddyfile --pidfile "$CADDY_DIR/pid"
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
    pattern=$(printf '%s|' "${LINKED_FILES[@]}" | sed 's/|$//; s/\./\\./g')
    git -C "$dir" status --porcelain 2>/dev/null | grep -cvE "^\?\? ($pattern)$" || true
}
