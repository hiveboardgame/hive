#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat << USAGE
usage: hive-cleanup [<name>...]
Lists worktrees, managed databases, proxy routes and optional Claude project leftovers,
lets you pick which to remove, and asks before removing. Names given as arguments are
preselected. Without arguments, clean worktrees whose commits are already in origin/main,
or whose branch head is part of a pull request merged into main on $GITHUB_REPO
(needs gh), are preselected. Branches are never deleted; $MAIN_DB, $MAIN_TEST_DB and
$TEMPLATE_DB never appear.
USAGE
}

declare -A HAS_DIR HAS_ENV HAS_DB HAS_TEST_DB HAS_ROUTE HAS_CLAUDE PORT DIRTY
declare -A DIR_BYTES DB_BYTES TEST_DB_BYTES
declare -A ORPHAN_DB MERGED
NAMES=()
SUGGESTED=()
MERGE_NOTES=()

note_name() {
    local n
    for n in "${NAMES[@]}"; do [ "$n" = "$1" ] && return; done
    NAMES+=("$1")
}

inventory() {
    local d name db claude_prefix databases identity role expected
    local -A known_dbs=()
    for d in "$WT_ROOT"/*/; do
        [ -d "$d" ] || continue
        name=$(basename "$d")
        [ "$name" = caddy ] && continue
        [ -f "$d$ENV_FILE" ] || worktree_registered "${d%/}" || continue
        note_name "$name"
        HAS_DIR[$name]=1
        if [ -f "$d$ENV_FILE" ]; then
            HAS_ENV[$name]=1
            PORT[$name]=$(sed -n 's/^WT_PORT=//p' "$d$ENV_FILE")
            # Existing instances predate database ownership comments.
            db=$(db_name "$name")
            if ! is_protected_db "$db" && db_exists "$db"; then
                HAS_DB[$name]=1
                known_dbs[$db]=1
            fi
            db=$(test_db_name "$name")
            if ! is_protected_db "$db" && db_exists "$db"; then
                HAS_TEST_DB[$name]=1
                known_dbs[$db]=1
            fi
        fi
        DIRTY[$name]=$(worktree_dirty_count "$d")
    done
    databases=$(worktree_dbs)
    while IFS='|' read -r db identity; do
        [ -n "$db" ] || continue
        [ -z "${known_dbs[$db]:-}" ] || continue
        if [ -z "$identity" ]; then
            # Legacy ownership identifies the checkout, but not the instance or role.
            name="db:$db"
            note_name "$name"
            ORPHAN_DB[$name]=$db
            continue
        fi
        name=${identity%:*}
        role=${identity##*:}
        expected=""
        case $role in
            dev) expected=$(db_name "$name") ;;
            test) expected=$(test_db_name "$name") ;;
        esac
        if [ -z "$name" ] || [ "$(sanitize_name "$name")" != "$name" ] || [ "$name" = "$MAIN_NAME" ] || [ "$db" != "$expected" ]; then
            warn "$db has inconsistent ownership metadata; review it manually"
            continue
        fi
        note_name "$name"
        if [ "$role" = dev ]; then HAS_DB[$name]=1; else HAS_TEST_DB[$name]=1; fi
    done <<< "$databases"
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

measure_sizes() {
    local name usage
    for name in "${NAMES[@]}"; do
        DIR_BYTES[$name]=0
        DB_BYTES[$name]=0
        TEST_DB_BYTES[$name]=0
        if [ -n "${HAS_DIR[$name]:-}" ]; then
            # du includes ignored build outputs, without following shared symlinks.
            usage=$(du -sk "$(wt_dir "$name")")
            DIR_BYTES[$name]=$((${usage%%[[:space:]]*} * 1024))
        fi
        if [ -n "${HAS_DB[$name]:-}" ]; then
            DB_BYTES[$name]=$(db_size_bytes "$(db_name "$name")")
        fi
        if [ -n "${ORPHAN_DB[$name]:-}" ]; then
            DB_BYTES[$name]=$(db_size_bytes "${ORPHAN_DB[$name]}")
        fi
        if [ -n "${HAS_TEST_DB[$name]:-}" ]; then
            TEST_DB_BYTES[$name]=$(db_size_bytes "$(test_db_name "$name")")
        fi
    done
}

check_merged() {
    local name dir branch prs number pr_head matched differs cherry gh_ok=1
    git -C "$(main_checkout)" fetch --quiet origin main 2> /dev/null ||
        warn "could not fetch origin/main; comparing against the last fetched copy"
    if ! command -v gh > /dev/null; then
        warn "gh is not installed; merged pull requests are not checked"
        gh_ok=0
    fi
    for name in "${NAMES[@]}"; do
        dir=$(wt_dir "$name")
        if [ -z "${HAS_DIR[$name]:-}" ] || ! worktree_registered "$dir"; then continue; fi
        branch=$(git -C "$dir" branch --show-current 2> /dev/null) || continue
        [ -n "$branch" ] || continue
        matched=""
        differs=""
        if [ "$gh_ok" = 1 ]; then
            if prs=$(gh pr list --repo "$GITHUB_REPO" --head "$branch" --base main --state merged \
                --json number,headRefOid --jq '.[] | "\(.number) \(.headRefOid)"' 2> /dev/null); then
                while read -r number pr_head; do
                    [ -n "$number" ] || continue
                    if git -C "$dir" merge-base --is-ancestor HEAD "$pr_head" 2> /dev/null; then
                        matched=$number
                        break
                    fi
                    differs=${differs:-$number}
                done <<< "$prs"
            else
                warn "gh could not list pull requests (offline or not logged in?); merged pull requests are not checked"
                gh_ok=0
            fi
        fi
        cherry=$(git -C "$dir" cherry origin/main HEAD 2> /dev/null) || cherry=""
        if [ -n "$matched" ]; then
            MERGED[$name]="PR #$matched"
        elif [ -n "$cherry" ] && ! grep -q '^+' <<< "$cherry"; then
            MERGED[$name]="into main"
        elif [ -n "$differs" ]; then
            MERGED[$name]="PR #$differs, differs"
            MERGE_NOTES+=("$name: PR #$differs is merged, but could not confirm it contains this worktree's commits; check before removing")
            continue
        else
            continue
        fi
        if [ "${DIRTY[$name]:-0}" -gt 0 ]; then
            MERGE_NOTES+=("$name is merged but has ${DIRTY[$name]} uncommitted change(s); not preselected")
        else
            SUGGESTED+=("$name")
        fi
    done
}

format_bytes() {
    local bytes=$1 unit scale=1
    for unit in B KiB MiB GiB TiB PiB; do
        if [ "$bytes" -lt "$((scale * 1024))" ] || [ "$unit" = PiB ]; then
            printf '%d.%d %s\n' "$((bytes / scale))" "$((bytes % scale * 10 / scale))" "$unit"
            return
        fi
        scale=$((scale * 1024))
    done
}

describe() {
    local name=$1 wt merged db tdb files total port route claude
    if [ -n "${ORPHAN_DB[$name]:-}" ]; then
        wt="database only (legacy)"
    elif [ -n "${HAS_DIR[$name]:-}" ]; then
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
    merged=${MERGED[$name]:--}
    files=$([ -n "${HAS_DIR[$name]:-}" ] && format_bytes "${DIR_BYTES[$name]}" || echo "-")
    db=$([ -n "${HAS_DB[$name]:-${ORPHAN_DB[$name]:-}}" ] && format_bytes "${DB_BYTES[$name]}" || echo "-")
    tdb=$([ -n "${HAS_TEST_DB[$name]:-}" ] && format_bytes "${TEST_DB_BYTES[$name]}" || echo "-")
    total=$(format_bytes "$((DIR_BYTES[$name] + DB_BYTES[$name] + TEST_DB_BYTES[$name]))")
    port="-"
    if [ -n "${PORT[$name]:-}" ]; then
        port=":${PORT[$name]}"
        [[ $(instance_status "$name" "${PORT[$name]}") == running* ]] && port="$port RUNNING"
    fi
    route=$([ -n "${HAS_ROUTE[$name]:-}" ] && echo yes || echo "-")
    claude=$([ -n "${HAS_CLAUDE[$name]:-}" ] && echo yes || echo "-")
    printf '%-18s %-26s %-17s %-10s %-10s %-10s %-10s %-16s %-6s %s\n' "$name" "$wt" "$merged" "$files" "$db" "$tdb" "$total" "$port" "$route" "$claude"
}

choose() {
    local preselected=$1
    if command -v gum > /dev/null; then
        gum choose --no-limit --show-help --header "Select instances to remove" --selected="$preselected" "${NAMES[@]}"
    else
        local i reply defaults="" picks=()
        for i in "${!NAMES[@]}"; do
            printf '%2d) %s\n' "$((i + 1))" "${NAMES[$i]}" >&2
            if [[ ,$preselected, == *,"${NAMES[$i]}",* ]]; then
                defaults+="${defaults:+ }$((i + 1))"
            fi
        done
        read -r -p "numbers to remove (space separated, 'all', or 'none')${defaults:+ [$defaults]}: " reply || return 130
        reply=${reply:-$defaults}
        [ "$reply" != none ] || return 0
        [ "$reply" = all ] && {
            printf '%s\n' "${NAMES[@]}"
            return
        }
        for i in $reply; do
            [[ $i =~ ^[0-9]+$ ]] && [ "$i" -ge 1 ] && [ "$i" -le "${#NAMES[@]}" ] || die "bad choice: $i"
            i=$((10#$i - 1))
            [[ " ${picks[*]} " == *" ${NAMES[$i]} "* ]] || picks+=("${NAMES[$i]}")
        done
        if [ "${#picks[@]}" -gt 0 ]; then printf '%s\n' "${picks[@]}"; fi
    fi
}

stop_instance() {
    local name=$1 port=$2 pid parent pids i p alive
    pid=$(listener_pid "$port") || return 0
    [ -n "$pid" ] || return 0
    if ! listener_is_instance "$pid" "$(wt_dir "$name")"; then
        warn ":$port is held by pid $pid, which is not $name; leaving it alone"
        return 0
    fi
    pids=("$pid")
    parent=$(ps -o ppid= -p "$pid" | tr -d ' ')
    [[ $(ps -o comm= -p "$parent" 2> /dev/null) == *cargo-leptos* ]] && pids=("$parent" "$pid")
    confirm "$name is running on :$port (pid ${pids[*]}); stop it?" || die "not stopping $name; aborting"
    kill "${pids[@]}" 2> /dev/null || true
    for i in $(seq 1 20); do
        alive=0
        for p in "${pids[@]}"; do kill -0 "$p" 2> /dev/null && alive=1; done
        [ "$alive" = 0 ] && return 0
        sleep 0.5
    done
    die "pid ${pids[*]} still alive after 10 s; stop $name before cleaning it up"
}

remove_name() {
    local name=$1 dir main project
    if [ -n "${ORPHAN_DB[$name]:-}" ]; then
        drop_db "${ORPHAN_DB[$name]}"
        log "dropped ${ORPHAN_DB[$name]}"
        return
    fi
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
    if [ -n "${HAS_DB[$name]:-}" ]; then
        drop_db "$(db_name "$name")"
        log "dropped $(db_name "$name")"
    fi
    if [ -n "${HAS_TEST_DB[$name]:-}" ]; then
        drop_db "$(test_db_name "$name")"
        log "dropped $(test_db_name "$name")"
    fi
    rm -f "$CADDY_DIR/share/$name"
    if [ -n "${HAS_CLAUDE[$name]:-}" ]; then
        project=$(claude_project_dir "$dir")
        [ -L "$project/memory" ] && rm "$project/memory"
        rmdir "$project" 2> /dev/null || log "kept $project (session transcripts)"
    fi
    return 0
}

main() {
    case ${1:-} in -h | --help)
        usage
        exit 0
        ;;
    esac
    require_repo
    require_commands git psql dropdb caddy curl lsof du
    inventory
    if [ ${#NAMES[@]} -eq 0 ]; then
        log "nothing to clean up"
        exit 0
    fi
    local n preselected="" selection picks=() name found total_files=0 total_dbs=0 total note
    for n in "$@"; do
        found=0
        for name in "${NAMES[@]}"; do
            if [ "$name" = "$n" ]; then
                preselected="${preselected:+$preselected,}$n"
                found=1
                break
            fi
        done
        [ "$found" = 1 ] || die "no cleanup candidate named '$n'"
    done

    log "checking which branches are merged..."
    check_merged
    if [ $# -eq 0 ] && [ ${#SUGGESTED[@]} -gt 0 ]; then
        preselected=$(
            IFS=,
            echo "${SUGGESTED[*]}"
        )
    fi
    log "measuring worktree files and databases..."
    measure_sizes
    printf '%-18s %-26s %-17s %-10s %-10s %-10s %-10s %-16s %-6s %s\n' NAME WORKTREE MERGED FILES DB TEST-DB TOTAL PORT ROUTE CLAUDE >&2
    for name in "${NAMES[@]}"; do describe "$name" >&2; done
    echo >&2
    for note in "${MERGE_NOTES[@]}"; do warn "$note"; done
    if [ ${#SUGGESTED[@]} -gt 0 ]; then
        note="merged with no uncommitted changes: ${SUGGESTED[*]}"
        [ $# -gt 0 ] || note+=" (preselected)"
        log "$note"
    fi
    selection=$(choose "$preselected") || {
        case $? in
            130)
                log "cancelled"
                exit 0
                ;;
            *) die "selection failed; nothing removed" ;;
        esac
    }
    [ -n "$selection" ] || {
        log "nothing selected"
        exit 0
    }
    mapfile -t picks <<< "$selection"

    for name in "${picks[@]}"; do
        total_files=$((total_files + DIR_BYTES[$name]))
        total_dbs=$((total_dbs + DB_BYTES[$name] + TEST_DB_BYTES[$name]))
    done
    total=$(format_bytes "$((total_files + total_dbs))")

    echo >&2
    log "this removes, for: ${picks[*]}"
    log "  the selected worktree directories and databases, Caddy routes, and optional Claude memory links"
    log "  branches are kept"
    log "  estimated size: $total (files: $(format_bytes "$total_files"), databases: $(format_bytes "$total_dbs"))"
    log "  actual space freed can differ with snapshots, compression, or shared file blocks"
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
    log "done; removed an estimated $total of worktree files and databases"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    main "$@"
fi
