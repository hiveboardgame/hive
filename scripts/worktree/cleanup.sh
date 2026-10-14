# shellcheck shell=bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat <<USAGE
usage: cleanup [<name>...]
Lists every worktree, database, proxy route and Claude project dir the setup left behind,
lets you pick which to remove, and asks before removing. Names given as arguments are
preselected. Branches are never deleted; $MAIN_DB, $MAIN_TEST_DB and $TEMPLATE_DB never appear.
USAGE
}

declare -A HAS_DIR HAS_ENV HAS_DB HAS_TEST_DB HAS_ROUTE HAS_CLAUDE PORT DIRTY
NAMES=()

note_name() {
    local n
    for n in "${NAMES[@]}"; do [ "$n" = "$1" ] && return; done
    NAMES+=("$1")
}

inventory() {
    local d name db claude_prefix
    for d in "$WT_ROOT"/*/; do
        [ -d "$d" ] || continue
        name=$(basename "$d")
        [ "$name" = caddy ] && continue
        note_name "$name"
        HAS_DIR[$name]=1
        if [ -f "$d$ENV_FILE" ]; then
            HAS_ENV[$name]=1
            PORT[$name]=$(sed -n 's/^WT_PORT=//p' "$d$ENV_FILE")
        fi
        DIRTY[$name]=$(worktree_dirty_count "$d")
    done
    for db in $(worktree_dbs); do
        name=${db#hive-}
        if [[ $name == *-test ]] && [ -z "${HAS_DIR[$name]:-}" ]; then
            name=${name%-test}
            note_name "$name"
            HAS_TEST_DB[$name]=1
        else
            note_name "$name"
            HAS_DB[$name]=1
        fi
    done
    for name in $(caddy_routes); do
        note_name "$name"
        HAS_ROUTE[$name]=1
    done
    claude_prefix=$(claude_project_dir "$WT_ROOT")-
    for d in "$claude_prefix"*/; do
        [ -d "$d" ] || continue
        name=${d#"$claude_prefix"}
        name=${name%/}
        note_name "$name"
        HAS_CLAUDE[$name]=1
    done
}

describe() {
    local name=$1 wt db tdb port route claude
    if [ -n "${HAS_DIR[$name]:-}" ]; then
        if worktree_registered "$(wt_dir "$name")"; then
            wt="worktree"
            [ "${DIRTY[$name]}" -gt 0 ] && wt="worktree, ${DIRTY[$name]} dirty"
        else
            wt="dir (not a worktree)"
        fi
        [ -n "${HAS_ENV[$name]:-}" ] || wt="$wt, no env"
    else
        wt="-"
    fi
    db=$([ -n "${HAS_DB[$name]:-}" ] && db_size "$(db_name "$name")" || echo "-")
    tdb=$([ -n "${HAS_TEST_DB[$name]:-}" ] && echo yes || echo "-")
    port="-"
    if [ -n "${PORT[$name]:-}" ]; then
        port=":${PORT[$name]}"
        [[ $(instance_status "$name" "${PORT[$name]}") == running* ]] && port="$port RUNNING"
    fi
    route=$([ -n "${HAS_ROUTE[$name]:-}" ] && echo yes || echo "-")
    claude=$([ -n "${HAS_CLAUDE[$name]:-}" ] && echo yes || echo "-")
    printf '%-18s %-26s %-9s %-7s %-16s %-6s %s\n' "$name" "$wt" "$db" "$tdb" "$port" "$route" "$claude"
}

choose() {
    local preselected=$1
    if command -v gum >/dev/null; then
        gum choose --no-limit --header "space selects, enter confirms" ${preselected:+--selected="$preselected"} "${NAMES[@]}"
    else
        local i reply picks=()
        for i in "${!NAMES[@]}"; do printf '%2d) %s\n' "$((i + 1))" "${NAMES[$i]}" >&2; done
        read -r -p "numbers to remove (space separated, or 'all'): " reply
        [ "$reply" = all ] && { printf '%s\n' "${NAMES[@]}"; return; }
        for i in $reply; do
            [[ $i =~ ^[0-9]+$ ]] && [ "$i" -ge 1 ] && [ "$i" -le "${#NAMES[@]}" ] || die "bad choice: $i"
            picks+=("${NAMES[$((i - 1))]}")
        done
        printf '%s\n' "${picks[@]}"
    fi
}

stop_instance() {
    local name=$1 port=$2 pid parent pids i p alive
    pid=$(listener_pid "$port")
    [ -n "$pid" ] || return 0
    if ! listener_is_instance "$pid" "$(wt_dir "$name")"; then
        warn ":$port is held by pid $pid, which is not $name; leaving it alone"
        return 0
    fi
    pids=("$pid")
    parent=$(ps -o ppid= -p "$pid" | tr -d ' ')
    [[ $(ps -o comm= -p "$parent" 2>/dev/null) == *cargo-leptos* ]] && pids=("$parent" "$pid")
    confirm "$name is running on :$port (pid ${pids[*]}); stop it?" || die "not stopping $name; aborting"
    kill "${pids[@]}" 2>/dev/null || true
    for i in $(seq 1 20); do
        alive=0
        for p in "${pids[@]}"; do kill -0 "$p" 2>/dev/null && alive=1; done
        [ "$alive" = 0 ] && return 0
        sleep 0.5
    done
    warn "pid ${pids[*]} still alive after 10 s"
}

remove_name() {
    local name=$1 dir main project
    main=$(main_checkout)
    dir=$(wt_dir "$name")
    [ -n "${PORT[$name]:-}" ] && stop_instance "$name" "${PORT[$name]}"
    if [ -n "${HAS_DIR[$name]:-}" ]; then
        if worktree_registered "$dir"; then
            git -C "$main" worktree remove --force "$dir"
        else
            [[ $dir == "$WT_ROOT"/* ]] || die "refusing to delete $dir outside $WT_ROOT"
            rm -rf "$dir"
        fi
        log "removed $dir"
    fi
    [ -n "${HAS_DB[$name]:-}" ] && drop_db "$(db_name "$name")" && log "dropped $(db_name "$name")"
    [ -n "${HAS_TEST_DB[$name]:-}" ] && drop_db "$(test_db_name "$name")" && log "dropped $(test_db_name "$name")"
    if [ -n "${HAS_CLAUDE[$name]:-}" ]; then
        project=$(claude_project_dir "$dir")
        [ -L "$project/memory" ] && rm "$project/memory"
        rmdir "$project" 2>/dev/null || log "kept $project (session transcripts)"
    fi
    return 0
}

main() {
    case ${1:-} in -h|--help) usage; exit 0 ;; esac
    require_repo
    inventory
    if [ ${#NAMES[@]} -eq 0 ]; then
        log "nothing to clean up"
        exit 0
    fi
    local n preselected="" picks=() name
    for n in "$@"; do
        for name in "${NAMES[@]}"; do [ "$name" = "$n" ] && preselected="${preselected:+$preselected,}$n"; done
    done

    printf '%-18s %-26s %-9s %-7s %-16s %-6s %s\n' NAME WORKTREE DB TEST-DB PORT ROUTE CLAUDE >&2
    for name in "${NAMES[@]}"; do describe "$name" >&2; done
    echo >&2
    mapfile -t picks < <(choose "$preselected")
    [ ${#picks[@]} -gt 0 ] || { log "nothing selected"; exit 0; }

    echo >&2
    log "this removes, for: ${picks[*]}"
    log "  the worktree directory, hive-<name> and hive-<name>-test, the Caddy route, the Claude memory link"
    log "  branches are kept"
    for name in "${picks[@]}"; do
        if [ "${DIRTY[$name]:-0}" -gt 0 ]; then
            warn "$name has ${DIRTY[$name]} uncommitted change(s) that will be lost:"
            git -C "$(wt_dir "$name")" status --short >&2
            read -r -p "type '$name' to confirm: " n
            [ "$n" = "$name" ] || die "aborted"
        fi
    done
    confirm "proceed?" || exit 1

    for name in "${picks[@]}"; do remove_name "$name"; done
    git -C "$(main_checkout)" worktree prune
    caddy_reload_if_running
    log "done"
}

main "$@"
