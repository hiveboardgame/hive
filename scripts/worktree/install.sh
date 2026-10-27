#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=/dev/null
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

usage() {
    cat << 'USAGE'
usage: ./scripts/worktree/install.sh [--bin-dir <directory>]
Installs native hive-setup, hive-run, and hive-cleanup launchers in ~/.local/bin by default.
The launchers use the main checkout's current scripts; existing unrelated commands
are never overwritten. Run from the main checkout or one of its worktrees.
USAGE
}

main() {
    local bin_dir="$HOME/.local/bin" command target scripts_dir installer_tmp
    while [ $# -gt 0 ]; do
        case $1 in
            -h | --help)
                usage
                exit 0
                ;;
            --bin-dir)
                [ $# -ge 2 ] || die "--bin-dir needs a directory"
                bin_dir=$2
                shift 2
                ;;
            *) die "unexpected argument $1" ;;
        esac
    done
    [ -n "$bin_dir" ] || die "--bin-dir must not be empty"
    require_repo
    require_commands bash cmp mktemp mkdir cat chmod rm
    scripts_dir="$MAIN_DIR/scripts/worktree"
    for command in setup run cleanup; do
        [ -f "$scripts_dir/$command.sh" ] || die "missing $scripts_dir/$command.sh; install from a main checkout with the worktree scripts"
    done

    installer_tmp=$(mktemp -d)
    trap 'rm -rf "$installer_tmp"' EXIT
    for command in setup run cleanup; do
        {
            printf '#!/usr/bin/env bash\n'
            printf 'export WT_LIB=%q\n' "$scripts_dir"
            printf 'export HIVE_WORKTREE_RUNTIME=native\n'
            printf 'exec bash %q "$@"\n' "$scripts_dir/$command.sh"
        } > "$installer_tmp/$command"
        target="$bin_dir/hive-$command"
        if [ -e "$target" ] || [ -L "$target" ]; then
            if [ -L "$target" ] || [ ! -f "$target" ] || ! cmp -s "$installer_tmp/$command" "$target"; then
                die "refusing to overwrite $target; choose another --bin-dir or move the existing command yourself"
            fi
        fi
    done

    mkdir -p "$bin_dir"
    for command in setup run cleanup; do
        target="$bin_dir/hive-$command"
        if [ ! -e "$target" ]; then
            # Do not overwrite a command created after the initial conflict check.
            (
                set -o noclobber
                cat "$installer_tmp/$command" > "$target"
            )
        fi
        chmod 755 "$target"
        log "installed $target"
    done
    rm -rf "$installer_tmp"
    trap - EXIT
    case :$PATH: in
        *:"$bin_dir":*) ;;
        *) log "add $bin_dir to your shell's PATH to use hive-setup, hive-run, and hive-cleanup" ;;
    esac
}

main "$@"
