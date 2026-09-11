#!/usr/bin/env bash
# Hive blue-green production deploy.
#
# End-to-end: detects which slot nginx is currently routing to, pulls latest
# code, lints new migrations for backward-incompat changes, builds the release
# into a per-slot directory, then swaps the active systemd slot behind nginx
# with no HTTP downtime. WebSocket clients are dropped when the outgoing slot
# stops and reconnect to the new one; see "The drain window" in README.md.

set -euo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

BINARY="$PROJECT_ROOT/.cargo/target/release/apis"
SITE_BUILD_DIR="$PROJECT_ROOT/target/site"

cd "$PROJECT_ROOT"
take_deploy_lock

SLOT=$(read_active_slot) || exit 1
read -r ACTIVE ACTIVE_PORT <<< "$SLOT"
case "$ACTIVE" in
    blue)  IDLE=green; IDLE_PORT=3001 ;;
    green) IDLE=blue;  IDLE_PORT=3000 ;;
    none)  IDLE=blue;  IDLE_PORT=3000
           echo "WARNING: no usable upstream in $UPSTREAM_FILE — treating as bootstrap." >&2 ;;
    *)     echo "ERROR: read_active_slot returned \"$SLOT\"" >&2; exit 1 ;;
esac
ACTIVE_PORT=${ACTIVE_PORT:-}

# Sanity: if nginx points at a colour, that colour's service must actually be
# running. Otherwise we'd cheerfully deploy to the "idle" slot — which is the
# one nginx is *not* talking to — and never realise traffic is already broken.
if [ "$ACTIVE" != "none" ] && ! systemctl is-active --quiet "hive@$ACTIVE"; then
    echo "ERROR: nginx upstream points to $ACTIVE (port $ACTIVE_PORT) but hive@$ACTIVE is not active." >&2
    echo "       Manual cleanup required — refusing to auto-recover." >&2
    exit 1
fi

echo "→ Active: $ACTIVE; deploying to: $IDLE (port $IDLE_PORT)"

IDLE_STARTED=0
FLIPPED=0
cleanup() {
    local rc=$?
    [ "$rc" -eq 0 ] && return
    # Only before the flip. Afterwards $IDLE is the slot nginx is routing to,
    # and stopping it would turn a failed run into an outage. Before it, leaving
    # it up means two instances indefinitely — the overlap hazard the advisory
    # locks only bound for a few seconds.
    if [ "$IDLE_STARTED" -eq 1 ] && [ "$FLIPPED" -eq 0 ]; then
        echo "→ Deploy failed before the flip; stopping $IDLE." >&2
        sudo systemctl stop "hive@$IDLE" 2>/dev/null || true
    fi
    echo "→ nginx is routing to: $(read_active_slot)" >&2
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

PRE_PULL_SHA=$(git rev-parse HEAD)
# The README's deep-rollback route is `git checkout <old-sha>` then re-deploy,
# which leaves a detached HEAD that `git pull` cannot work with — under set -e
# that would kill the deploy it is meant to enable.
if BRANCH=$(git symbolic-ref --quiet --short HEAD); then
    echo "→ Pulling latest on $BRANCH..."
    git pull --ff-only
else
    echo "→ Detached HEAD at ${PRE_PULL_SHA:0:12}; deploying it as-is, not pulling." >&2
fi
POST_PULL_SHA=$(git rev-parse HEAD)

# --- Migration safety lint ------------------------------------------------
# Migrations run on app boot (apis/src/main.rs). During the swap the OLD
# binary keeps serving against the NEW schema, so anything destructive here
# breaks live traffic. Refuse unless explicitly overridden.
ACTIVE_SHA_FILE="$(slot_dir "$ACTIVE")/sha"
COMPARE_FROM=""
if [ "$ACTIVE" != "none" ] && [ -f "$ACTIVE_SHA_FILE" ]; then
    COMPARE_FROM=$(cat "$ACTIVE_SHA_FILE")
elif [ "$PRE_PULL_SHA" != "$POST_PULL_SHA" ]; then
    COMPARE_FROM="$PRE_PULL_SHA"
fi
if [ -n "$COMPARE_FROM" ]; then
    # Fail closed. A diff that errors — SHA garbage-collected, force-pushed
    # away, shallow clone — says nothing about whether the schema changed, and
    # swallowing it here means the destructive-migration check never runs.
    if ! MIGRATION_DIFF=$(git diff --name-only --diff-filter=AM "$COMPARE_FROM"..HEAD -- 'db/migrations/' 2>&1); then
        echo "ERROR: cannot diff $COMPARE_FROM..HEAD for migrations:" >&2
        echo "$MIGRATION_DIFF" | sed 's/^/    /' >&2
        echo "       Refusing to deploy without knowing whether the schema changed." >&2
        exit 1
    fi
    NEW_MIGRATION_FILES=$(printf '%s\n' "$MIGRATION_DIFF" | grep -E '/up\.sql$' || true)
    if [ -n "$NEW_MIGRATION_FILES" ]; then
        echo "→ New migrations since active deploy:"
        echo "$NEW_MIGRATION_FILES" | sed 's/^/    /'
        # Fail closed here too. `grep -l` exits 1 for "no match" but 2 for "could
        # not read", and collapsing those with `|| true` lets a migration that
        # nobody managed to read pass as backward-compatible.
        while IFS= read -r m; do
            [ -r "$m" ] || {
                echo "ERROR: migration $m is listed but not readable; cannot lint it." >&2
                exit 1
            }
        done <<< "$NEW_MIGRATION_FILES"
        set +e
        DESTRUCTIVE=$(printf '%s\n' "$NEW_MIGRATION_FILES" | xargs -r -d '\n' grep -liE \
            'DROP[[:space:]]+(COLUMN|TABLE|INDEX|CONSTRAINT|TYPE|SEQUENCE)|ALTER[[:space:]]+(TABLE|COLUMN)[[:space:]].+(RENAME|TYPE|SET[[:space:]]+NOT[[:space:]]+NULL)|TRUNCATE')
        GREP_RC=$?
        set -e
        if [ "$GREP_RC" -gt 1 ]; then
            echo "ERROR: the migration lint could not read its inputs (grep exit $GREP_RC)." >&2
            exit 1
        fi
        if [ -n "$DESTRUCTIVE" ]; then
            echo "" >&2
            echo "ERROR: migration(s) appear non-backward-compatible:" >&2
            echo "$DESTRUCTIVE" | sed 's/^/    /' >&2
            echo "" >&2
            echo "Blue/green needs the OLD binary to keep working against the NEW schema." >&2
            echo "If this really is safe (e.g. dropping a column nothing reads), set" >&2
            echo "ALLOW_DESTRUCTIVE_MIGRATION=1 and re-run." >&2
            if [ "${ALLOW_DESTRUCTIVE_MIGRATION:-}" != "1" ]; then
                exit 1
            fi
            echo "→ ALLOW_DESTRUCTIVE_MIGRATION=1; proceeding." >&2
        fi
    fi
fi

# --- Backup, in parallel with the build ----------------------------------
# The old deploy.sh dumped before every deploy; keep that, but overlap it with
# the build so it costs no extra wall clock. It has to finish before the idle
# slot boots, because booting is what applies the migrations.
BACKUP_DIR="$PROJECT_ROOT/db-backups"
BACKUP_FILE="$BACKUP_DIR/$(date +%F-%H:%M:%S).dump"
BACKUP_PID=
if [ "${SKIP_BACKUP:-}" = "1" ]; then
    echo "→ SKIP_BACKUP=1; not dumping the database."
else
    DB_URL=$(sed -n 's/^DATABASE_URL=//p' "$PROD_ENV" | head -1)
    if [ -z "$DB_URL" ]; then
        echo "ERROR: no DATABASE_URL in $PROD_ENV; cannot back up." >&2
        echo "       Set SKIP_BACKUP=1 to deploy without one." >&2
        exit 1
    fi
    mkdir -p "$BACKUP_DIR"
    echo "→ Dumping database to $BACKUP_FILE (in background)..."
    pg_dump -Fc -d "$DB_URL" -f "$BACKUP_FILE" &
    BACKUP_PID=$!
fi

echo "→ Building release binary + assets..."
# nice: the live slot is serving on the same 4 physical cores.
nice -n 10 env LEPTOS_HASH_FILES=true cargo leptos build -rP

if [ ! -x "$BINARY" ]; then
    echo "ERROR: build did not produce $BINARY" >&2
    exit 1
fi
if [ ! -d "$SITE_BUILD_DIR/pkg" ]; then
    echo "ERROR: build did not produce $SITE_BUILD_DIR/pkg" >&2
    exit 1
fi

# leptos reads this at render time and panics if it is missing while
# LEPTOS_HASH_FILES=true, so a wrong guess here has to fail the deploy rather
# than ship a slot that 500s on every page.
HASH_SRC=
for candidate in \
    "$(dirname "$BINARY")/hash.txt" \
    "$SITE_BUILD_DIR/hash.txt" \
    "$PROJECT_ROOT/hash.txt" \
    "$PROJECT_ROOT/target/hash.txt"
do
    [ -f "$candidate" ] && { HASH_SRC="$candidate"; break; }
done
if [ -z "$HASH_SRC" ]; then
    echo "ERROR: LEPTOS_HASH_FILES=true but no hash.txt was produced by the build." >&2
    echo "       Looked beside $BINARY, in $SITE_BUILD_DIR, and in $PROJECT_ROOT." >&2
    echo "       Find it with: find $PROJECT_ROOT -name hash.txt -newermt '-10 minutes'" >&2
    exit 1
fi
echo "→ Hash manifest: $HASH_SRC"

# --- Stage the release ----------------------------------------------------
# Built in a scratch dir and moved into place, so nothing is ever copied over
# the running binary — an interrupted deploy that left the idle slot up would
# otherwise fail here with ETXTBSY and need manual cleanup.
SLOT_DIR=$(slot_dir "$IDLE")
STAGE="$SLOT_DIR.new"
rm -rf "$STAGE" "$SLOT_DIR.old"
mkdir -p "$STAGE"
cp "$BINARY" "$STAGE/hive"
chmod +x "$STAGE/hive"
cp "$HASH_SRC" "$STAGE/hash.txt"
cp -a "$SITE_BUILD_DIR" "$STAGE/site"
git rev-parse HEAD > "$STAGE/sha"

# Record this release's own files before absorbing anything, so the next deploy
# carries over one generation rather than the accumulated union of every
# release that ever ran.
( cd "$STAGE/site/pkg" && find . -type f | sort ) > "$STAGE/pkg.manifest"

if [ "$ACTIVE" != "none" ]; then
    carry_over_assets "$(slot_dir "$ACTIVE")" "$STAGE/site/pkg"
    echo "  carried over the outgoing release's hashed assets"
fi

if [ -n "$BACKUP_PID" ]; then
    echo "→ Waiting for the database dump..."
    if ! wait "$BACKUP_PID"; then
        echo "ERROR: pg_dump failed; refusing to deploy without a backup." >&2
        echo "       Set SKIP_BACKUP=1 to override." >&2
        rm -f "$BACKUP_FILE"
        exit 1
    fi
    echo "  backup OK ($(du -h "$BACKUP_FILE" | cut -f1))"
    # One dump per deploy fills the disk otherwise, and a full disk is its
    # own outage. Newest kept; override with BACKUP_KEEP.
    # Names are `%F-%H:%M:%S.dump`, zero-padded throughout, so a lexicographic
    # sort is a chronological one — and this avoids parsing `ls` output.
    if ! find "$BACKUP_DIR" -maxdepth 1 -type f -name '*.dump' \
        | sort -r | tail -n "+$(( ${BACKUP_KEEP:-14} + 1 ))" \
        | while IFS= read -r old_dump; do rm -f "$old_dump"; done
    then
        echo "WARNING: could not prune old dumps in $BACKUP_DIR" >&2
    fi
fi

sudo systemctl stop "hive@$IDLE" 2>/dev/null || true
[ -d "$SLOT_DIR" ] && mv "$SLOT_DIR" "$SLOT_DIR.old"
mv "$STAGE" "$SLOT_DIR"
rm -rf "$SLOT_DIR.old"


sudo systemctl start "hive@$IDLE"
IDLE_STARTED=1

verify_slot "$IDLE" "$IDLE_PORT"
swap_upstream "$IDLE" "$IDLE_PORT"
FLIPPED=1

if [ "$ACTIVE" = "none" ]; then
    echo "→ Bootstrap complete. Serving from $IDLE."
    exit 0
fi

echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
sleep "$DRAIN_SECONDS"

sudo systemctl stop "hive@$ACTIVE"
echo "→ Done. Active: $IDLE on port $IDLE_PORT."
