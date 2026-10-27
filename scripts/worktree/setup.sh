#!/usr/bin/env bash
set -Eeuo pipefail
# shellcheck source=/dev/null
source "${WT_LIB:-$(dirname "${BASH_SOURCE[0]}")}/lib.sh"

usage() {
    cat << USAGE
usage:
  hive-setup <branch> [--from <ref>] [--seed-template <database>]
                                  worktree + databases + port + route for <branch>
  hive-setup --template <dump> [--name <database>] [--scrubbed]
                                  rebuild a template (default $TEMPLATE_DB)
  hive-setup --refresh <name>           replace a worktree database using its saved template
  hive-setup --ls                       list instances

<branch> is used as-is if it exists locally or on origin; otherwise it is created from
<ref> (default origin/main). Instance name = branch with everything but [a-z0-9-] as '-'.
Templates must be named hive-template or hive-template-<name>.
--scrubbed skips scrubbing; use only for dumps already scrubbed by a trusted process.
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
    local dump=$1 database=$2 status=0
    if pg_restore -l "$dump" > /dev/null 2>&1; then
        local toc
        toc=$(mktemp)
        pg_restore -l "$dump" | grep -vE ' (COMMENT - )?EXTENSION ' > "$toc"
        pg_restore --exit-on-error -L "$toc" --no-owner --no-privileges --role="$PG_USER" -j4 -h "$PG_HOST" -U "$PG_USER" -d "$database" "$dump" || status=$?
        rm -f "$toc"
        return "$status"
    elif [[ $dump == *.gz ]]; then
        gunzip -c "$dump" | psql_db "$database"
    else
        psql_db "$database" -f "$dump"
    fi
}

cmd_template() {
    local dump=$1 template=$2 scrubbed=$3 staging="hive-template-restore-$$" created_staging=0
    validate_template_name "$template"
    [ -f "$dump" ] || die "no such file: $dump"
    require_createdb
    if db_exists "$template"; then
        confirm "$template exists; drop and rebuild it from $dump?" || exit 1
    fi
    db_exists "$staging" && die "temporary database $staging already exists"
    template_failed() {
        trap - ERR INT TERM
        log "template restore failed; $template was not published"
        if [ "$created_staging" = 1 ]; then
            dropdb --if-exists --force -h "$PG_HOST" -U "$PG_USER" "$staging"
        fi
        exit 1
    }
    trap template_failed ERR INT TERM
    create_db "$staging"
    created_staging=1
    log "restoring $dump for $template"
    restore_dump "$dump" "$staging"
    if [ "$scrubbed" = 0 ]; then
        log "scrubbing"
        psql_db "$staging" -f "$WT_LIB/scrub.sql"
    fi
    psql_db "$staging" -c 'vacuum analyze'
    psql_db "$staging" -tAc 'select count(*) from users; select max(version) from __diesel_schema_migrations'
    # Keep an existing seed usable until its replacement has restored and validated.
    if db_exists "$template"; then
        terminate_connections "$template"
        dropdb --force -h "$PG_HOST" -U "$PG_USER" "$template"
    fi
    psql_admin -v staging="$staging" -v target="$template" << 'SQL'
ALTER DATABASE :"staging" RENAME TO :"target";
SQL
    trap - ERR INT TERM
    log "$template ready: $(db_size "$template")"
}

cmd_refresh() {
    local name=$1
    [ "$name" != "$MAIN_NAME" ] || die "refusing to refresh $MAIN_DB"
    load_env "$name"
    db_exists "$WT_SEED_TEMPLATE" || die "no $WT_SEED_TEMPLATE; restore it with hive-setup --template <dump> --name $WT_SEED_TEMPLATE"
    [[ $(instance_status "$name" "$WT_PORT") == running* ]] && die "$name is running on :$WT_PORT; stop it first"
    confirm "drop $WT_DB and clone it again from $WT_SEED_TEMPLATE?" || exit 1
    drop_db "$WT_DB"
    create_db -T "$WT_SEED_TEMPLATE" "$WT_DB"
    mark_worktree_db "$WT_DB" "$name" dev
    log "$WT_DB refreshed ($(db_size "$WT_DB"))"
}

add_exclude() {
    local file line
    file="$(git -C "$(main_checkout)" rev-parse --path-format=absolute --git-common-dir)/info/exclude"
    for line in "$@"; do
        grep -qx "$line" "$file" 2> /dev/null || echo "$line" >> "$file"
    done
}

share_local_files() {
    local dir=$1 main f
    main=$(main_checkout)
    for f in "${LOCAL_FILES[@]}"; do
        [ -e "$main/$f" ] || continue
        { [ -e "$dir/$f" ] || [ -L "$dir/$f" ]; } && continue
        mkdir -p "$dir/$(dirname "$f")"
        if [ "$f" = .agents ]; then
            # Codex protects .agents in its sandbox; keep it inside this checkout.
            cp -RL "$main/$f" "$dir/$f"
        else
            ln -s "$main/$f" "$dir/$f"
        fi
    done
}

cmd_new() {
    local branch=$1 from=$2 seed=$3 name dir db test_db main port
    validate_template_name "$seed"
    name=$(sanitize_name "$branch")
    [ -n "$name" ] || die "branch name '$branch' sanitizes to nothing"
    [ "$name" != "$MAIN_NAME" ] || die "'$MAIN_NAME' is the main checkout"
    main=$(main_checkout)
    dir=$(wt_dir "$name")
    db=$(db_name "$name")
    test_db=$(test_db_name "$name")
    is_protected_db "$db" && die "$db is reserved"
    [ "${#test_db}" -le 63 ] || die "branch name is too long for a database name"
    [ ! -e "$dir" ] || die "$dir already exists"
    [ ! -f "$(env_path "$name")" ] || die "instance $name already exists"
    db_exists "$db" && die "database $db already exists (hive-cleanup first)"
    db_exists "$test_db" && die "database $test_db already exists (hive-cleanup first)"
    db_exists "$seed" || die "no $seed; run: hive-setup --template <dump> --name $seed"
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
        git -C "$main" worktree add --no-track -b "$branch" "$dir" "$from"
        created_branch=1
    fi
    created_wt=1

    local f excludes=("/$ENV_FILE")
    for f in "${LOCAL_FILES[@]}"; do excludes+=("/$f"); done
    add_exclude "${excludes[@]}"
    share_local_files "$dir"
    if [ "$WORKTREE_RUNTIME" = nix ]; then
        direnv allow "$dir"
    fi

    create_db -T "$seed" "$db"
    created_db=1
    mark_worktree_db "$db" "$name" dev
    create_db "$test_db"
    created_test_db=1
    mark_worktree_db "$test_db" "$name" test

    port=$(alloc_port)
    write_env "$name" "$branch" "$port" "$seed"
    caddy_ensure

    if uses_claude; then
        local project memory
        project=$(claude_project_dir "$dir")
        memory=$(main_memory_dir)
        mkdir -p "$project" "$memory"
        [ -e "$project/memory" ] || ln -s "$memory" "$project/memory"
    fi

    trap - ERR
    cat << DONE

$name is ready
  worktree   $dir  ($branch)
  databases  $db ($(db_size "$db")), $test_db
  port       $port  ->  $(origin_for "$name")

next:
  hive-run $name                        serve it (first build takes a while)
  cd $dir                         open the worktree in your editor or agent
Safari and curl need this line in /etc/hosts (Chrome and Firefox do not):
  127.0.0.1 $name.localhost
DONE
}

main() {
    [ $# -gt 0 ] || {
        usage
        exit 1
    }
    case $1 in -h | --help)
        usage
        exit 0
        ;;
    esac
    require_repo
    case $1 in
        --ls)
            require_commands psql lsof
            cmd_ls
            ;;
        --template)
            [ $# -ge 2 ] || die "--template needs a dump file"
            local dump=$2 template=$TEMPLATE_DB scrubbed=0
            shift 2
            while [ $# -gt 0 ]; do
                case $1 in
                    --name)
                        [ $# -ge 2 ] || die "--name needs a database"
                        template=$2
                        shift 2
                        ;;
                    --scrubbed)
                        scrubbed=1
                        shift
                        ;;
                    *) die "unexpected argument $1" ;;
                esac
            done
            require_commands psql createdb dropdb pg_restore gunzip
            cmd_template "$dump" "$template" "$scrubbed"
            ;;
        --refresh)
            [ $# -eq 2 ] || die "usage: hive-setup --refresh <name>"
            require_commands psql createdb dropdb lsof
            cmd_refresh "$2"
            ;;
        --*) die "unknown option $1" ;;
        *)
            local branch=$1 from=origin/main seed=$TEMPLATE_DB
            shift
            while [ $# -gt 0 ]; do
                case $1 in
                    --from)
                        [ $# -ge 2 ] || die "--from needs a ref"
                        from=$2
                        shift 2
                        ;;
                    --seed-template)
                        [ $# -ge 2 ] || die "--seed-template needs a database"
                        seed=$2
                        shift 2
                        ;;
                    *) die "unexpected argument $1" ;;
                esac
            done
            require_runtime
            if [ "$WORKTREE_RUNTIME" = nix ]; then require_commands direnv; fi
            require_commands git psql createdb dropdb caddy curl lsof
            cmd_new "$branch" "$from" "$seed"
            ;;
    esac
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    main "$@"
fi
