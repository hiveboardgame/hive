#!/usr/bin/env bash
# Hive blue-green rollback.
#
# Brings the previously-deployed slot back online. Refuses to roll back if any
# new migrations were introduced between the rollback target's commit and the
# currently-active commit (rolling back a binary against a migrated DB is a
# good way to corrupt data). Pass --force to override.

set -euo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

FORCE=
[ "${1:-}" = "--force" ] && FORCE=1

cd "$PROJECT_ROOT"
take_deploy_lock

SLOT=$(read_active_slot) || exit 1
read -r ACTIVE _ <<< "$SLOT"
case "$ACTIVE" in
    blue)  TARGET=green; TARGET_PORT=3001 ;;
    green) TARGET=blue;  TARGET_PORT=3000 ;;
    *)
        echo "ERROR: no active slot to roll back from (upstream said \"$SLOT\")." >&2
        exit 1
        ;;
esac
if ! systemctl is-active --quiet "hive@$ACTIVE"; then
    echo "ERROR: nginx points to $ACTIVE but hive@$ACTIVE is not active — manual cleanup required." >&2
    exit 1
fi

TARGET_DIR=$(slot_dir "$TARGET")
TARGET_BIN="$TARGET_DIR/hive"
TARGET_SHA_FILE="$TARGET_DIR/sha"
ACTIVE_SHA_FILE="$(slot_dir "$ACTIVE")/sha"

if [ ! -x "$TARGET_BIN" ]; then
    echo "ERROR: rollback target binary not found at $TARGET_BIN." >&2
    echo "       (Have you deployed at least twice? Rollback only works one step back.)" >&2
    exit 1
fi

# --- Migration safety check -----------------------------------------------
# A `git diff` that *fails* (target SHA garbage-collected, force-pushed away,
# shallow clone) tells us nothing about migrations, so it has to be treated
# the same as finding some — not as an all-clear.
if [ -f "$TARGET_SHA_FILE" ] && [ -f "$ACTIVE_SHA_FILE" ]; then
    TARGET_SHA=$(cat "$TARGET_SHA_FILE")
    ACTIVE_SHA=$(cat "$ACTIVE_SHA_FILE")
    echo "→ Active: $ACTIVE @ ${ACTIVE_SHA:0:12}"
    echo "→ Target: $TARGET @ ${TARGET_SHA:0:12}"

    UNSAFE=
    if MIGRATION_DIFF=$(git diff --name-only "$TARGET_SHA".."$ACTIVE_SHA" -- db/migrations/ 2>&1); then
        if [ -n "$MIGRATION_DIFF" ]; then
            echo "" >&2
            echo "ERROR: migrations were introduced between target and active:" >&2
            echo "$MIGRATION_DIFF" | sed 's/^/  /' >&2
            echo "" >&2
            echo "These migrations have already been applied to the database. Rolling back" >&2
            echo "to a binary that doesn't know about them risks data corruption." >&2
            UNSAFE=1
        fi
    else
        echo "ERROR: could not diff $TARGET_SHA..$ACTIVE_SHA:" >&2
        echo "$MIGRATION_DIFF" | sed 's/^/  /' >&2
        echo "Cannot tell whether migrations were introduced." >&2
        UNSAFE=1
    fi
    if [ -n "$UNSAFE" ]; then
        if [ -z "$FORCE" ]; then
            echo "Pass --force to override (only if you know what you're doing)." >&2
            exit 1
        fi
        echo "→ --force given; proceeding anyway." >&2
    fi
else
    echo "WARNING: missing SHA file(s) for $ACTIVE or $TARGET — cannot verify migration safety." >&2
    if [ -z "$FORCE" ]; then
        echo "Pass --force to roll back anyway." >&2
        exit 1
    fi
fi

if [ ! -f "$TARGET_DIR/hash.txt" ]; then
    echo "ERROR: $TARGET_DIR/hash.txt missing; the slot would panic on render." >&2
    exit 1
fi

echo "→ Rolling back from $ACTIVE to $TARGET (port $TARGET_PORT)..."

carry_over_assets "$(slot_dir "$ACTIVE")" "$TARGET_DIR/site/pkg"

TARGET_STARTED=0
FLIPPED=0
cleanup() {
    local rc=$?
    [ "$rc" -eq 0 ] && return
    # Only before the flip. Afterwards $TARGET is the slot nginx is routing to,
    # and stopping it would turn a failed run into an outage. Before it, leaving
    # it up means two instances indefinitely — the overlap hazard the advisory
    # locks only bound for a few seconds.
    if [ "$TARGET_STARTED" -eq 1 ] && [ "$FLIPPED" -eq 0 ]; then
        echo "→ Rollback failed before the flip; stopping $TARGET." >&2
        sudo systemctl stop "hive@$TARGET" 2>/dev/null || true
    fi
    echo "→ nginx is routing to: $(read_active_slot)" >&2
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

sudo systemctl start "hive@$TARGET"
TARGET_STARTED=1

verify_slot "$TARGET" "$TARGET_PORT"
swap_upstream "$TARGET" "$TARGET_PORT"
FLIPPED=1

echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
sleep "$DRAIN_SECONDS"

sudo systemctl stop "hive@$ACTIVE"
echo "→ Rollback complete. Active: $TARGET on port $TARGET_PORT."
