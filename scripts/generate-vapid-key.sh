#!/usr/bin/env bash
# Generate a VAPID signing key for Web Push (browser notifications) and print
# it as the single-line base64 value to paste into VAPID_PRIVATE_KEY.
#
#   ./scripts/generate-vapid-key.sh
#
# Then set it in the target environment's env file (NOT a file path):
#
#   VAPID_PRIVATE_KEY="<the base64 printed below>"
#   VAPID_SUBJECT="mailto:you@example.com"   # or https://hivegame.com
#
# The committed .env already ships a DEV/localhost key; use this to generate a
# real key for production (set it in the prod env file, never committed).
# The server derives the public key from this at startup — no public-key file
# to manage. Rotating the prod key invalidates every existing browser
# subscription.
set -euo pipefail

# base64 of the EC P-256 private-key PEM, one line (matches the decoder in
# apis/src/notifications/web_push.rs).
openssl ecparam -genkey -name prime256v1 -noout | openssl base64 -A
echo
