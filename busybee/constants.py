import discord
import os
import logging

COMMAND_PREFIX = "?"
BOT_TOKEN = os.environ["DISCORD_BOT_TOKEN"]
DISCORD_CLIENT_ID = os.environ["DISCORD_CLIENT_ID"]
DISCORD_CLIENT_SECRET = os.environ["DISCORD_CLIENT_SECRET"]

DISCORD_BOT_DATABASE_URL = "sqlite:///busybee.db"


# Will attempt to look for the user in each of these channels,
# and pings them in the first one it finds which they have access to
PING_CHANNELS_IDS = [1338963129664671818]

OAUTH_SECRET_EXPIRY = 20 * 60  # 20 minutes

ERROR_COLOR = discord.Color(0xFF0000)
BOT_NAME = "BusyBee"

REDIRECT_URI = os.environ.get(
    "BUSYBEE_API_REDIRECT_URI", "http://localhost:3000/oauth/callback"
)

WS_URL = "ws://localhost:8080/ws"
RETRY_TIMEOUT_SECONDS = 1

GUILD_ID = 1338963129664671815
