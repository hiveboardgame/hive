#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

MODE=deploy
case "${1:-}" in
    "")           ;;
    --bootstrap)  MODE=bootstrap ;;
    --build-only) MODE=build ;;
    *)            echo "usage: $0 [--bootstrap | --build-only]" >&2; exit 2 ;;
esac

require_drone
cd "$PROJECT_ROOT"
take_deploy_lock
start_run_log "$MODE"

ACTIVE=none
IDLE=
case "$MODE" in
    deploy)
        ACTIVE=$(require_consistent_state) || exit 1
        IDLE=$(other_slot "$ACTIVE")
        echo "→ Live: $ACTIVE; deploying to: $IDLE"
        ;;
    bootstrap)
        problems=()
        [ -n "${LIVE_SHA:-}" ] || problems+=("LIVE_SHA is not set (the commit the old process was built from)")
        [ -z "${LIVE_SHA:-}" ] || git cat-file -e "${LIVE_SHA}^{commit}" 2>/dev/null || problems+=("LIVE_SHA $LIVE_SHA is not a commit here")
        [ "$(read_active_slot)" = blue ] || problems+=("nginx must already route to the old process on :3000 (cutover step 3)")
        [ -n "$(ss -tlnH "sport = :3000")" ] || problems+=("nothing listens on :3000; the old process must still be running")
        systemctl is-active --quiet hive@blue && problems+=("hive@blue is running")
        systemctl is-active --quiet hive@green && problems+=("hive@green is running")
        [ -z "$(read_marker current)" ] || problems+=("already bootstrapped ($RELEASES_DIR/current exists)")
        if [ "${#problems[@]}" -gt 0 ]; then
            echo "ERROR: cannot bootstrap:" >&2
            printf '    - %s\n' "${problems[@]}" >&2
            print_status >&2
            exit 1
        fi
        IDLE=green
        echo "→ Bootstrap: old process on :3000 (built from ${LIVE_SHA:0:12}) → green"
        ;;
esac

IDLE_STARTED=0
VERIFIED=0
write_live_markers() { record_live_release "$SHA"; }
cleanup() {
    local rc=$?
    # Nothing may cut cleanup short: not a second Ctrl-C, not a dead logger.
    trap '' INT TERM HUP PIPE
    set +e
    [ "$rc" -eq 0 ] && return
    if [ "$IDLE_STARTED" -eq 1 ]; then
        recover_after_failure "$IDLE" "$ACTIVE" "$VERIFIED"
    fi
    echo "" >&2
    print_status >&2
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

PRE_PULL_SHA=$(git rev-parse HEAD)
if [ "$MODE" = bootstrap ]; then
    echo "→ Bootstrap deploys ${PRE_PULL_SHA:0:12} as checked at checkpoint A; not pulling."
elif BRANCH=$(git symbolic-ref --quiet --short HEAD); then
    echo "→ Pulling latest on $BRANCH..."
    git pull --ff-only
else
    echo "→ Detached HEAD at ${PRE_PULL_SHA:0:12}; deploying it as-is, not pulling." >&2
fi
SHA=$(git rev-parse HEAD)
RELEASE=$(release_dir "$SHA")

if [ "$MODE" = deploy ] && [ "$SHA" = "$(read_marker rolled-back)" ] && [ "${REDEPLOY_ROLLED_BACK:-}" != 1 ]; then
    echo "ERROR: ${SHA:0:12} was rolled back. Push a fix (or a revert) first," >&2
    echo "       or set REDEPLOY_ROLLED_BACK=1 if you really mean to ship it again." >&2
    exit 1
fi

if [ "$MODE" != build ]; then
    [ "$MODE" = bootstrap ] || LIVE_SHA=$(read_marker current)
    NEW_RC=0
    LIVE_RC=0
    check_schema_compat "$SHA" || NEW_RC=$?
    check_schema_compat "$LIVE_SHA" "$SHA" || LIVE_RC=$?
    if [ "$NEW_RC" -ge 2 ] || [ "$LIVE_RC" -ge 2 ]; then
        echo "Refusing to deploy without knowing whether both releases fit the schema." >&2
        exit 1
    fi
    if [ "$NEW_RC" -ne 0 ] || [ "$LIVE_RC" -ne 0 ]; then
        echo "Every migration must keep the previous release working and must not lock a busy" >&2
        echo "table for long. If these are fine, re-run with ALLOW_DESTRUCTIVE_MIGRATION=1." >&2
        if [ "${ALLOW_DESTRUCTIVE_MIGRATION:-}" != "1" ]; then
            exit 1
        fi
        echo "→ ALLOW_DESTRUCTIVE_MIGRATION=1; proceeding." >&2
    fi
fi

BACKUP_PID=
BACKUP_DIR="$PROJECT_ROOT/db-backups"
BACKUP_FILE="$BACKUP_DIR/$(date +%F-%H:%M:%S).dump"
if [ "$MODE" != build ]; then
    if [ "${SKIP_BACKUP:-}" = "1" ]; then
        echo "→ SKIP_BACKUP=1; not dumping the database."
    else
        mkdir -p "$BACKUP_DIR"
        chmod 700 "$BACKUP_DIR"
        FREE_GB=$(df --output=avail -BG "$BACKUP_DIR" | tail -1 | tr -dc 0-9)
        if [ "$FREE_GB" -lt 10 ]; then
            echo "ERROR: only ${FREE_GB}G free for the dump; a full disk takes Postgres down." >&2
            exit 1
        fi
        echo "→ Dumping database to $BACKUP_FILE (in background)..."
        (umask 077; exec 9>&-; pg_dump -Fc -d "service=$PG_SERVICE" -f "$BACKUP_FILE") &
        BACKUP_PID=$!
    fi
fi

if release_complete "$SHA"; then
    echo "→ Release ${SHA:0:12} already built; reusing it."
else
    git worktree prune
    if [ -d "$BUILD_DIR" ]; then
        git -C "$BUILD_DIR" checkout --quiet --detach --force "$SHA"
    else
        git worktree add --detach "$BUILD_DIR" "$SHA"
    fi

    echo "→ Building ${SHA:0:12} in $BUILD_DIR..."
    (cd "$BUILD_DIR" && nice -n 10 env LEPTOS_HASH_FILES=true HIVE_RELEASE_SHA="$SHA" cargo leptos build -rP)

    BINARY="$BUILD_DIR/.cargo/target/release/apis"
    HASH_FILE="$BUILD_DIR/.cargo/target/release/hash.txt"
    SITE="$BUILD_DIR/target/site"
    for required in "$BINARY" "$HASH_FILE" "$SITE/pkg"; do
        if [ ! -e "$required" ]; then
            echo "ERROR: the build did not produce $required" >&2
            exit 1
        fi
    done

    STAGE="$RELEASE.new"
    rm -rf "$STAGE" "$RELEASE"
    mkdir -p "$STAGE"
    cp "$BINARY" "$STAGE/hive"
    chmod +x "$STAGE/hive"
    cp "$HASH_FILE" "$STAGE/hash.txt"
    cp -a "$SITE" "$STAGE/site"
    echo "$SHA" > "$STAGE/sha"
    touch "$STAGE/.complete"
    mv "$STAGE" "$RELEASE"
    echo "→ Release staged at $RELEASE"
fi

if [ "$MODE" = build ]; then
    echo "→ Build only; nothing else changed."
    exit 0
fi

if [ -n "$BACKUP_PID" ]; then
    echo "→ Waiting for the database dump..."
    if ! wait "$BACKUP_PID"; then
        echo "ERROR: pg_dump failed; refusing to deploy without a backup." >&2
        echo "       Check ~/.pg_service.conf [$PG_SERVICE] and ~/.pgpass, or set SKIP_BACKUP=1." >&2
        rm -f "$BACKUP_FILE"
        exit 1
    fi
    echo "  backup OK ($(du -h "$BACKUP_FILE" | cut -f1))"
    if ! find "$BACKUP_DIR" -maxdepth 1 -type f -name '*.dump' \
        | sort -r | tail -n "+$(( ${BACKUP_KEEP:-14} + 1 ))" \
        | while IFS= read -r old_dump; do rm -f "$old_dump"; done
    then
        echo "WARNING: could not prune old dumps in $BACKUP_DIR" >&2
    fi
fi

echo "→ Migrations about to be applied:"
DATABASE_URL="service=$PG_SERVICE" "$RELEASE/hive" --pending-migrations | sed 's/^/    /'
# A migration waiting on a lock queues every app query behind it, so give up quickly instead.
PGOPTIONS="-c lock_timeout=${MIGRATION_LOCK_TIMEOUT:-10s} -c statement_timeout=${MIGRATION_STATEMENT_TIMEOUT:-300s}" \
    DATABASE_URL="service=$PG_SERVICE" "$RELEASE/hive" --migrate-only

sudo systemctl stop "hive@$IDLE" 2>/dev/null || true
point_slot "$IDLE" "$SHA"
IDLE_STARTED=1
sudo systemctl start "hive@$IDLE"

verify_slot "$IDLE" "$SHA"
swap_upstream "$IDLE"
verify_via_nginx "$SHA" "$IDLE"
VERIFIED=1

BOOT_OK=1
if [ "$ACTIVE" = none ]; then
    set_boot_slot "$IDLE" "" || BOOT_OK=0
    echo "→ Draining the old process for ${DRAIN_SECONDS}s..."
    sleep "$DRAIN_SECONDS"
    stop_old_process
else
    set_boot_slot "$IDLE" "$ACTIVE" || BOOT_OK=0
    echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
    sleep "$DRAIN_SECONDS"
    sudo systemctl stop "hive@$ACTIVE"
fi
set_active_instance "$IDLE"

record_live_release "$SHA"
prune_releases

if [ "$BOOT_OK" -ne 1 ]; then
    echo "ERROR: $IDLE is live, but the boot slot could not be switched." >&2
    echo "       Run: sudo systemctl enable hive@$IDLE" >&2
    [ "$ACTIVE" = none ] || echo "            sudo systemctl disable hive@$ACTIVE" >&2
    exit 1
fi
final_verdict "$IDLE" "$SHA"
