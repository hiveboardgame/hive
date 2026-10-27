#! /bin/bash

set -euo pipefail

echo RUNNING MIGRATIONS
cd /app/db && diesel migration run && cd ..

echo STARTING APP
export RUSTC_WRAPPER=/app/dev-container/serial_app_rustc.sh
cargo leptos watch --hot-reload
