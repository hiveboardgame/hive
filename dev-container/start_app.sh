#! /bin/bash

set -euo pipefail

release_mode=false
test_mode_docker="${TEST_MODE_DOCKER:-false}"
case "${1:-}" in
  "") ;;
  --release) release_mode=true ;;
  *)
    echo "Usage: $0 [--release]" >&2
    exit 2
    ;;
esac

case "$test_mode_docker" in
  false | "") ;;
  true)
    release_mode=true
    echo RUNNING TEST-MODE FORMAT CHECK
    cargo install leptosfmt
    cargo fmt -q --check
    leptosfmt apis -q --check
    ;;
  *)
    echo "TEST_MODE_DOCKER must be true or false." >&2
    exit 2
    ;;
esac

echo RUNNING MIGRATIONS
cd /app/db && diesel migration run && cd ..

if [[ "$test_mode_docker" == true ]]; then
  echo LOADING TESTWARE DATA
  psql "$DATABASE_URL" --no-psqlrc --set ON_ERROR_STOP=1 --single-transaction \
    --file /app/db/testware.sql
fi

echo ENSURING WASM TARGET
rustup target add wasm32-unknown-unknown

if [[ "$release_mode" == true ]]; then
  echo STARTING RELEASE APP
  exec cargo leptos serve --release
else
  echo STARTING APP
  exec cargo leptos watch --hot-reload
fi
