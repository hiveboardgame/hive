#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat << USAGE
usage: hive-run <name> [serve|watch|shell|evaluator] [--share] [--lan-ip <IPv4>] [--tailnet-ip <IPv4>]
  serve   cargo leptos serve behind $(origin_for '<name>') (default)
  watch   cargo leptos watch, hot reload on the instance's reload port
  shell   a shell in the worktree with the instance's environment
  evaluator  hive-evaluator for the running instance, with the engines in
          \${EVAL_ENGINES_FILE:-~/.config/hive-evaluator/engines.toml}; user requests only
          unless EVAL_AUTO=true
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

run_evaluator() {
    local name=$1 engines=${EVAL_ENGINES_FILE:-$HOME/.config/hive-evaluator/engines.toml} token
    [ -r "$engines" ] || die "no engines file at $engines; describe the engine there (hive-evaluator/README.md, Engines) or set EVAL_ENGINES_FILE"
    # The token the instance's site reads: its environment first, then the checkout's .env.
    token=${EVAL_WORKER_TOKEN:-$(sed -n 's/^EVAL_WORKER_TOKEN=//p' .env 2> /dev/null | tr -d "\"'" | head -1)}
    [ -n "$token" ] || die "the site has no EVAL_WORKER_TOKEN, so it refuses every worker; set one in $MAIN_DIR/$ENV_FILE and restart $name"
    curl_instance "$name" /health > /dev/null \
        || warn "$(origin_for "$name") is not answering; start it with 'hive-run $name' (the worker keeps trying)"
    export EVAL_WORKER_TOKEN=$token EVAL_ENGINES_FILE=$engines HIVE_URL="http://localhost:$WT_PORT"
    export EVAL_WORKER_NAME=${EVAL_WORKER_NAME:-local-$name} EVAL_AUTO=${EVAL_AUTO:-false}
    log "hive-evaluator for $name on :$WT_PORT, engines from $engines, auto $EVAL_AUTO"
    if [ "$WORKTREE_RUNTIME" = nix ]; then
        exec nix develop .#evaluator -c cargo run --release -p hive-evaluator
    fi
    exec cargo run --release -p hive-evaluator
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
            serve | watch | shell | evaluator)
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
    if { [ "$mode" = shell ] || [ "$mode" = evaluator ]; } && [ "$share" = 1 ]; then
        die "--share configures serve/watch; $mode uses the instance's saved sharing settings"
    fi
    case $mode in
        serve | watch) require_commands caddy curl lsof ;;
        evaluator) require_commands curl ;;
    esac
    if [ "$WORKTREE_RUNTIME" = native ] && [ "$mode" != shell ]; then
        require_commands cargo
        [ "$mode" = evaluator ] || require_commands cargo-leptos
    fi

    load_env "$name"
    [ -d "$WT_DIR" ] || die "$WT_DIR is missing"
    if [ "$mode" = shell ] || [ "$mode" = evaluator ]; then
        load_share_addresses "$name"
    else
        if [ "$share" = 1 ]; then detect_share_addresses "$lan_ip" "$tailnet_ip"; fi
        ensure_port_free "$name"
        [ "$name" = "$MAIN_NAME" ] || write_env "$name" "$WT_BRANCH" "$WT_PORT"
    fi
    export_instance_env "$name" "$WT_PORT" "$WT_DB" "$WT_TEST_DB"
    cd "$WT_DIR"
    if [ "$mode" = evaluator ]; then run_evaluator "$name"; fi

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
