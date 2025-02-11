import discord
import dataset
from discord.ext import commands

from constants import COMMAND_PREFIX, DISCORD_BOT_DATABASE_URL

def init():
    global bot
    global db 

    intents = discord.Intents.default()
    intents.members = True
    intents.guilds = True
    intents.message_content = True
    bot = commands.Bot(command_prefix=COMMAND_PREFIX, intents=intents)
    db = dataset.connect(DISCORD_BOT_DATABASE_URL)

