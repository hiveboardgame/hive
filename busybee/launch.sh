#!/bin/bash

python3 bot.py&
uvicorn api:app --host "${BUSYBEE_HOST:-0.0.0.0}" --port 8080&

# Wait for any process to exit
wait -n

# Exit with status of process that exited first
exit $?
