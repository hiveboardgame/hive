# shellcheck shell=bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat <<USAGE
usage: run <name> [serve|watch|shell]
  serve   cargo leptos serve behind $(origin_for '<name>') (default)
  watch   cargo leptos watch, hot reload on the instance's reload port
  shell   a nix shell in the worktree with the instance's environment
'$MAIN_NAME' is the main checkout on :$MAIN_PORT.
USAGE
}

ensure_port_free() {
    local name=$1 pid cwd
    port_listening "$WT_PORT" || return 0
    pid=$(listener_pid "$WT_PORT")
    cwd=$(listener_cwd "$pid")
    if listener_is_instance "$pid" "$WT_DIR"; then
        die "$name already runs on :$WT_PORT (pid $pid)"
    fi
    [ "$name" != "$MAIN_NAME" ] || die ":$MAIN_PORT is taken by pid $pid ($cwd)"
    local new
    new=$(alloc_port)
    warn ":$WT_PORT is taken by pid $pid ($cwd); moving $name to :$new"
    write_env "$name" "$WT_BRANCH" "$new"
    WT_PORT=$new
}

watch_health() {
    local name=$1 deadline=$((SECONDS + HEALTH_TIMEOUT_SECS)) body
    while [ $SECONDS -lt $deadline ]; do
        if body=$(curl_instance "$name" /health); then
            log "==> $(origin_for "$name") is up (/health: $body)"
            return
        fi
        sleep 2
    done
    warn "$(origin_for "$name") did not answer /health within $HEALTH_TIMEOUT_SECS s"
}

main() {
    [ $# -ge 1 ] && [ $# -le 2 ] || { usage; exit 1; }
    case $1 in -h|--help) usage; exit 0 ;; esac
    local name=$1 mode=${2:-serve}
    case $mode in serve|watch|shell) ;; *) die "unknown mode $mode" ;; esac
    require_repo

    load_env "$name"
    [ -d "$WT_DIR" ] || die "$WT_DIR is missing"
    [ "$mode" = shell ] || ensure_port_free "$name"
    [ "$name" = "$MAIN_NAME" ] || write_env "$name" "$WT_BRANCH" "$WT_PORT"
    export_instance_env "$name" "$WT_PORT" "$WT_DB" "$WT_TEST_DB"
    caddy_ensure
    cd "$WT_DIR"

    if [ "$mode" = shell ]; then
        log "shell for $name ($WT_BRANCH): $DATABASE_URL, :$WT_PORT"
        exec nix develop -c "${SHELL:-zsh}"
    fi
    log "$name ($WT_BRANCH) on :$WT_PORT -> $(origin_for "$name"), db $WT_DB"
    watch_health "$name" &
    exec nix develop -c cargo leptos "$mode"
}

main "$@"
