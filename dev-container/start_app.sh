#! /bin/bash

set -e
echo RUNNING MIGRATIONS
cd /app/db && diesel migration run && cd ..

echo STARTING APP
rustup target add wasm32-unknown-unknown
cargo leptos watch --hot-reload
