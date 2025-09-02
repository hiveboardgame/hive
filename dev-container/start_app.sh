#! /bin/bash

set -euo pipefail

echo RUNNING MIGRATIONS
cd /app/db && diesel migration run && cd ..

echo STARTING APP
cargo leptos watch --hot-reload
