#!/bin/bash
set -euo pipefail

compiler="${1:-rustc}"
if (($#)); then shift; fi

# Leptos starts separate frontend and server Cargo processes. Queue only the
# large app crates before rustc allocates memory; dependencies can still build
# in parallel.
previous=""
for argument in "$@"; do
    if [[ ("$previous" == "--crate-name" && "$argument" == "apis") || "$argument" == "--crate-name=apis" ]]; then
        mkdir -p /app/.cargo/target
        exec 9>/app/.cargo/target/apis-rustc.lock
        flock 9
        break
    fi
    previous="$argument"
done

exec "$compiler" "$@"
