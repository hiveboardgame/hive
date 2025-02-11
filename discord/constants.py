import discord
import os

COMMAND_PREFIX = "?"
BOT_TOKEN = os.environ["DISCORD_BOT_TOKEN"] 
DISCORD_CLIENT_ID = os.environ["DISCORD_CLIENT_ID"]
DISCORD_CLIENT_SECRET = os.environ["DISCORD_CLIENT_SECRET"]
DISCORD_BOT_DATABASE_URL = os.environ["DISCORD_BOT_DATABASE_URL"]

# Will attempt to look for the user in each of these channels,
# and pings them in the first one it finds which they have access to
PING_CHANNELS_IDS = [
    1326224200431697983
]

OAUTH_SECRET_EXPIRY = 20 * 60 # 20 minutes

ERROR_COLOR = discord.Color(0xFF0000)
BOT_NAME = "Hive Game Discord Bot"

REDIRECT_URI = "http://localhost:8080/oauth/callback"

WS_URL = "ws://localhost:8080/ws"
RETRY_TIMEOUT_SECONDS = 1

GUILD_ID = 1326224200431697980 
