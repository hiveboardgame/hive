import discord
import dataset
import os
import logging
import coloredlogs
from discord.ext import commands

from constants import COMMAND_PREFIX, DISCORD_BOT_DATABASE_URL

logger = logging.getLogger(__name__)
global initiated
initialized = False

def init():
    global bot
    global db 
    global initialized
    coloredlogs.install(level="INFO")

    if initialized:
        logger.warning("Config already initialized, skipping...")
        return

    if "REDIRECT_URI" not in os.environ:
        logger.warning("BUSYBEE_API_REDIRECT_URI not set in environment variables, using default")


    # Auto-create a database in the working directory if it does not yet exist
    # Make sure the working directory is not ephemeral!
    if "busybee.db" not in os.listdir():
        logger.warning("Database not found, will create new database when new data needs to be saved (./busybee.db)...")

    intents = discord.Intents.default()
    intents.members = True
    intents.guilds = True
    intents.message_content = True
    bot = commands.Bot(command_prefix=COMMAND_PREFIX, intents=intents)
    db = dataset.connect(DISCORD_BOT_DATABASE_URL)
    initialized = True

