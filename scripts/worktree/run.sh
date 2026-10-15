#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat << USAGE
usage: hive-run <name> [serve|watch|shell] [--share] [--lan-ip <IPv4>] [--tailnet-ip <IPv4>]
  serve   cargo leptos serve behind $(origin_for '<name>') (default)
  watch   cargo leptos watch, hot reload on the instance's reload port
  shell   a shell in the worktree with the instance's environment
  --share print LAN and Tailnet URLs using detected IPv4 addresses and sslip.io
  --lan-ip / --tailnet-ip override address detection when sharing
'$MAIN_NAME' is the main checkout on :$MAIN_PORT.
Uses local tools by default; HIVE_WORKTREE_RUNTIME=nix enters the worktree's nix shell.
USAGE
}

ensure_port_free() {
    local name=$1 port pid cwd conflict=""
    for port in "$WT_PORT" "$((WT_PORT + 1))"; do
        port_listening "$port" || continue
        pid=$(listener_pid "$port")
        cwd=$(listener_cwd "$pid")
        if listener_is_instance "$pid" "$WT_DIR"; then
            die "$name already runs on :$port (pid $pid)"
        fi
        [ "$name" != "$MAIN_NAME" ] || die ":$port is taken by pid $pid ($cwd)"
        conflict+=", :$port is taken by pid $pid ($cwd)"
    done
    [ -n "$conflict" ] || return 0
    local new
    new=$(alloc_port)
    warn "${conflict#, }; moving $name to :$new / :$((new + 1))"
    write_env "$name" "$WT_BRANCH" "$new"
    WT_PORT=$new
}

watch_health() {
    local name=$1 deadline=$((SECONDS + HEALTH_TIMEOUT_SECS)) body runner_pid=$$
    while kill -0 "$runner_pid" 2> /dev/null && [ $SECONDS -lt $deadline ]; do
        if body=$(curl_instance "$name" /health); then
            log "==> $(origin_for "$name") is up (/health: $body)"
            return
        fi
        sleep 2
    done
    if kill -0 "$runner_pid" 2> /dev/null; then
        warn "$(origin_for "$name") did not answer /health within $HEALTH_TIMEOUT_SECS s"
    fi
}

print_instance_urls() {
    local name=$1 index
    printf '%-8s %s\n' Local: "$(origin_for "$name")" >&2
    for index in "${!SHARE_IPS[@]}"; do
        printf '%-8s %s\n' "${SHARE_LABELS[$index]}:" "$(shared_origin_for "$name" "${SHARE_IPS[$index]}")" >&2
    done
}

main() {
    [ $# -ge 1 ] || {
        usage
        exit 1
    }
    case $1 in -h | --help)
        usage
        exit 0
        ;;
    esac
    local name=$1 mode=serve mode_set=0 share=0 lan_ip="" tailnet_ip="" active
    shift
    while [ $# -gt 0 ]; do
        case $1 in
            --share)
                share=1
                shift
                ;;
            --lan-ip)
                [ $# -ge 2 ] || die "--lan-ip needs an IPv4 address"
                lan_ip=$2
                shift 2
                ;;
            --tailnet-ip)
                [ $# -ge 2 ] || die "--tailnet-ip needs an IPv4 address"
                tailnet_ip=$2
                shift 2
                ;;
            serve | watch | shell)
                [ "$mode_set" = 0 ] || die "only one run mode may be selected"
                mode=$1
                mode_set=1
                shift
                ;;
            *) die "unexpected argument $1" ;;
        esac
    done
    if [ "$share" = 0 ] && { [ -n "$lan_ip" ] || [ -n "$tailnet_ip" ]; }; then
        die "address overrides require --share"
    fi
    require_repo
    require_runtime
    for active in "${HIVE_WORKTREE:-}" "${WT_NAME:-}"; do
        if [ -n "$active" ] && [ "$active" != "$name" ]; then
            die "leave the $active environment before running $name"
        fi
    done
    if [ "$mode" = shell ] && [ "$share" = 1 ]; then
        die "--share configures serve/watch; shell uses the instance's saved sharing settings"
    fi
    if [ "$mode" != shell ]; then require_commands caddy curl lsof; fi
    if [ "$WORKTREE_RUNTIME" = native ] && [ "$mode" != shell ]; then
        require_commands cargo cargo-leptos
    fi

    load_env "$name"
    [ -d "$WT_DIR" ] || die "$WT_DIR is missing"
    if [ "$mode" = shell ]; then
        load_share_addresses "$name"
    else
        if [ "$share" = 1 ]; then detect_share_addresses "$lan_ip" "$tailnet_ip"; fi
        ensure_port_free "$name"
        [ "$name" = "$MAIN_NAME" ] || write_env "$name" "$WT_BRANCH" "$WT_PORT"
    fi
    export_instance_env "$name" "$WT_PORT" "$WT_DB" "$WT_TEST_DB"
    cd "$WT_DIR"

    if [ "$mode" = shell ]; then
        log "shell for $name ($WT_BRANCH), db $WT_DB, :$WT_PORT"
        if [ "$WORKTREE_RUNTIME" = nix ]; then
            exec nix develop -c "${SHELL:-bash}"
        fi
        exec "${SHELL:-bash}"
    fi
    save_share_addresses "$name"
    caddy_ensure
    print_instance_urls "$name"
    log "$name ($WT_BRANCH) on :$WT_PORT -> $(origin_for "$name"), db $WT_DB"
    watch_health "$name" &
    if [ "$WORKTREE_RUNTIME" = nix ]; then
        exec nix develop -c cargo leptos "$mode"
    fi
    exec cargo leptos "$mode"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    main "$@"
fi
