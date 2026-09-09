#! /bin/bash

set -euo pipefail

test_mode_docker="${TEST_MODE_DOCKER:-false}"

case "$test_mode_docker" in
  false | "") ;;
  true)
    echo RUNNING TEST-MODE FORMAT CHECK
    cargo fmt -q --check
    leptosfmt apis -q --check
    ;;
  *)
    echo "TEST_MODE_DOCKER must be true or false." >&2
    exit 2
    ;;
esac

echo RUNNING MIGRATIONS
(
  cd /app/db
  diesel migration run

  if [[ "$test_mode_docker" == true ]]; then
    echo RUNNING TESTWARE MIGRATIONS
    diesel migration run --migration-dir testware
  fi
)

if [[ "$test_mode_docker" == true ]]; then
  # Exercise the production build profiles in Docker test mode.
  echo STARTING TEST APP
  exec cargo leptos serve --release
else
  echo STARTING APP
  exec cargo leptos watch --hot-reload
fi
