# shellcheck shell=bash
set -euo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat <<USAGE
usage:
  setup <branch> [--from <ref>]   worktree + databases + port + route for <branch>
  setup --template <dump>         (re)build $TEMPLATE_DB from a pg_dump file, scrubbed
  setup --refresh <name>          drop hive-<name> and clone it again from $TEMPLATE_DB
  setup --ls                      list instances

<branch> is used as-is if it exists locally or on origin; otherwise it is created from
<ref> (default origin/main). Instance name = branch with everything but [a-z0-9-] as '-'.
USAGE
}

cmd_ls() {
    local name status
    printf '%-18s %-28s %-6s %-26s %-9s %-20s %s\n' NAME BRANCH PORT DATABASE SIZE STATUS URL
    for name in "$MAIN_NAME" $(instances); do
        load_env "$name"
        status=$(instance_status "$name" "$WT_PORT")
        printf '%-18s %-28s %-6s %-26s %-9s %-20s %s\n' "$name" "$WT_BRANCH" "$WT_PORT" "$WT_DB" \
            "$(db_exists "$WT_DB" && db_size "$WT_DB" || echo missing)" "$status" "$(origin_for "$name")"
    done
}

restore_dump() {
    local dump=$1 status=0
    if pg_restore -l "$dump" >/dev/null 2>&1; then
        local toc
        toc=$(mktemp)
        pg_restore -l "$dump" | grep -vE ' (COMMENT - )?EXTENSION ' > "$toc"
        pg_restore -L "$toc" --no-owner --no-privileges --role="$PG_USER" -j4 -h "$PG_HOST" -U "$PG_USER" -d "$TEMPLATE_DB" "$dump" || status=$?
        rm -f "$toc"
        [ "$status" -eq 0 ] || warn "pg_restore exited with $status; check the messages above"
    elif [[ $dump == *.gz ]]; then
        gunzip -c "$dump" | psql_db "$TEMPLATE_DB"
    else
        psql_db "$TEMPLATE_DB" -f "$dump"
    fi
}

cmd_template() {
    local dump=$1
    [ -f "$dump" ] || die "no such file: $dump"
    require_createdb
    if db_exists "$TEMPLATE_DB"; then
        confirm "$TEMPLATE_DB exists; drop and rebuild it from $dump?" || exit 1
        terminate_connections "$TEMPLATE_DB"
        dropdb --force -h "$PG_HOST" -U "$PG_USER" "$TEMPLATE_DB"
    fi
    create_db "$TEMPLATE_DB"
    log "restoring $dump into $TEMPLATE_DB"
    restore_dump "$dump"
    log "scrubbing"
    psql_db "$TEMPLATE_DB" -f "$WT_LIB/scrub.sql"
    psql_db "$TEMPLATE_DB" -c 'vacuum analyze'
    log "$TEMPLATE_DB ready: $(db_size "$TEMPLATE_DB"), $(psql_db "$TEMPLATE_DB" -tAc 'select count(*) from users') users, migration $(psql_db "$TEMPLATE_DB" -tAc 'select max(version) from __diesel_schema_migrations')"
}

cmd_refresh() {
    local name=$1
    [ "$name" != "$MAIN_NAME" ] || die "refusing to refresh $MAIN_DB"
    load_env "$name"
    db_exists "$TEMPLATE_DB" || die "no $TEMPLATE_DB; run: setup --template <dump>"
    [[ $(instance_status "$name" "$WT_PORT") == running* ]] && die "$name is running on :$WT_PORT; stop it first"
    confirm "drop $WT_DB and clone it again from $TEMPLATE_DB?" || exit 1
    drop_db "$WT_DB"
    create_db -T "$TEMPLATE_DB" "$WT_DB"
    log "$WT_DB refreshed ($(db_size "$WT_DB"))"
}

add_exclude() {
    local file line
    file="$(git -C "$(main_checkout)" rev-parse --git-common-dir)/info/exclude"
    for line in "$@"; do
        grep -qx "$line" "$file" 2>/dev/null || echo "$line" >> "$file"
    done
}

link_shared_files() {
    local dir=$1 main f
    main=$(main_checkout)
    for f in "${LINKED_FILES[@]}"; do
        [ -e "$main/$f" ] || continue
        [ -e "$dir/$f" ] && continue
        mkdir -p "$dir/$(dirname "$f")"
        ln -s "$main/$f" "$dir/$f"
    done
}

cmd_new() {
    local branch=$1 from=$2 name dir db test_db main port
    name=$(sanitize_name "$branch")
    [ -n "$name" ] || die "branch name '$branch' sanitizes to nothing"
    [ "$name" != "$MAIN_NAME" ] || die "'$MAIN_NAME' is the main checkout"
    main=$(main_checkout)
    dir=$(wt_dir "$name")
    db=$(db_name "$name")
    test_db=$(test_db_name "$name")
    [ ! -e "$dir" ] || die "$dir already exists"
    [ ! -f "$(env_path "$name")" ] || die "instance $name already exists"
    db_exists "$db" && die "database $db already exists (cleanup first)"
    db_exists "$test_db" && die "database $test_db already exists (cleanup first)"
    db_exists "$TEMPLATE_DB" || die "no $TEMPLATE_DB; run: setup --template <dump>"
    require_createdb

    local created_branch=0 created_wt=0 created_db=0 created_test_db=0
    undo() {
        trap - ERR
        log "setup failed; undoing"
        [ "$created_test_db" = 1 ] && drop_db "$test_db"
        [ "$created_db" = 1 ] && drop_db "$db"
        [ "$created_wt" = 1 ] && git -C "$main" worktree remove --force "$dir"
        [ "$created_branch" = 1 ] && git -C "$main" branch -D "$branch"
        rm -f "$(env_path "$name")"
        caddy_reload_if_running
        exit 1
    }
    trap undo ERR

    mkdir -p "$WT_ROOT"
    log "fetching origin"
    git -C "$main" fetch --quiet --prune origin
    if git -C "$main" show-ref --verify --quiet "refs/heads/$branch"; then
        log "using local branch $branch"
        git -C "$main" worktree add "$dir" "$branch"
    elif git -C "$main" show-ref --verify --quiet "refs/remotes/origin/$branch"; then
        log "tracking origin/$branch"
        git -C "$main" worktree add --track -b "$branch" "$dir" "origin/$branch"
        created_branch=1
    else
        log "creating $branch from $from"
        git -C "$main" worktree add -b "$branch" "$dir" "$from"
        created_branch=1
    fi
    created_wt=1

    local f excludes=("/$ENV_FILE")
    for f in "${LINKED_FILES[@]}"; do excludes+=("/$f"); done
    add_exclude "${excludes[@]}"
    link_shared_files "$dir"
    direnv allow "$dir"

    create_db -T "$TEMPLATE_DB" "$db"
    created_db=1
    create_db "$test_db"
    created_test_db=1

    port=$(alloc_port)
    write_env "$name" "$branch" "$port"
    caddy_ensure

    if uses_claude; then
        local project memory
        project=$(claude_project_dir "$dir")
        memory=$(main_memory_dir)
        mkdir -p "$project" "$memory"
        [ -e "$project/memory" ] || ln -s "$memory" "$project/memory"
    fi

    trap - ERR
    cat <<DONE

$name is ready
  worktree   $dir  ($branch)
  databases  $db ($(db_size "$db")), $test_db
  port       $port  ->  $(origin_for "$name")

next:
  run $name                        serve it (first build takes a while)
  cd $dir && claude                a Claude on this branch
Safari and curl need this line in /etc/hosts (Chrome and Firefox do not):
  127.0.0.1 $name.localhost
DONE
}

main() {
    [ $# -gt 0 ] || { usage; exit 1; }
    case $1 in -h|--help) usage; exit 0 ;; esac
    require_repo
    case $1 in
        --ls) cmd_ls ;;
        --template) [ $# -eq 2 ] || die "usage: setup --template <dump>"; cmd_template "$2" ;;
        --refresh) [ $# -eq 2 ] || die "usage: setup --refresh <name>"; cmd_refresh "$2" ;;
        --*) die "unknown option $1" ;;
        *)
            local branch=$1 from=origin/main
            shift
            while [ $# -gt 0 ]; do
                case $1 in
                    --from) [ $# -ge 2 ] || die "--from needs a ref"; from=$2; shift 2 ;;
                    *) die "unexpected argument $1" ;;
                esac
            done
            cmd_new "$branch" "$from"
            ;;
    esac
}

main "$@"
