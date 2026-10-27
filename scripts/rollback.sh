#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

FORCE=
case "${1:-}" in
    "")      ;;
    --force) FORCE=1 ;;
    *)       echo "usage: $0 [--force]" >&2; exit 2 ;;
esac

require_drone
cd "$PROJECT_ROOT"
take_deploy_lock
start_run_log rollback

ACTIVE=$(require_consistent_state) || exit 1
TARGET=$(other_slot "$ACTIVE")
CURRENT=$(read_marker current)
PREVIOUS=$(read_marker previous)

if ! release_complete "$PREVIOUS"; then
    echo "ERROR: no previous release to roll back to (marker: '${PREVIOUS:-none}')." >&2
    exit 1
fi
echo "→ Live: $ACTIVE @ ${CURRENT:0:12}; rolling back to ${PREVIOUS:0:12} on $TARGET"

SCHEMA_RC=0
check_schema_compat "$PREVIOUS" || SCHEMA_RC=$?
if [ "$SCHEMA_RC" -ne 0 ]; then
    echo "The previous release may not work against the schema in the database." >&2
    if [ -z "$FORCE" ]; then
        echo "Pass --force to roll back anyway." >&2
        exit 1
    fi
    echo "→ --force given; proceeding." >&2
fi

TARGET_STARTED=0
VERIFIED=0
write_live_markers() {
    echo "$PREVIOUS" > "$RELEASES_DIR/current"
    echo "$CURRENT" > "$RELEASES_DIR/previous"
    echo "$CURRENT" > "$RELEASES_DIR/rolled-back"
}
cleanup() {
    local rc=$?
    # Nothing may cut cleanup short: not a second Ctrl-C, not a dead logger.
    trap '' INT TERM HUP PIPE
    set +e
    [ "$rc" -eq 0 ] && return
    if [ "$TARGET_STARTED" -eq 1 ]; then
        recover_after_failure "$TARGET" "$ACTIVE" "$VERIFIED"
    fi
    echo "" >&2
    print_status >&2
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

point_slot "$TARGET" "$PREVIOUS"
TARGET_STARTED=1
sudo systemctl start "hive@$TARGET"

verify_slot "$TARGET" "$PREVIOUS"
swap_upstream "$TARGET"
verify_via_nginx "$PREVIOUS" "$TARGET"
VERIFIED=1

BOOT_OK=1
set_boot_slot "$TARGET" "$ACTIVE" || BOOT_OK=0
echo "→ Draining $ACTIVE for ${DRAIN_SECONDS}s..."
sleep "$DRAIN_SECONDS"
sudo systemctl stop "hive@$ACTIVE"
set_active_instance "$TARGET"
write_live_markers

if [ "$BOOT_OK" -ne 1 ]; then
    echo "ERROR: $TARGET is live, but the boot slot could not be switched." >&2
    echo "       Run: sudo systemctl enable hive@$TARGET && sudo systemctl disable hive@$ACTIVE" >&2
    exit 1
fi
final_verdict "$TARGET" "$PREVIOUS"
