from discord.ext import commands
import discord
from utils import pretty_print

import sys
import traceback
import config

from models import *
from constants import GUILD_ID

@config.bot.tree.command(
        name="enable_pings",
        description = "Enable pings from HiveGame.com to be sent to user.",
        guild = discord.Object(id=GUILD_ID)
) 
async def enable_pings(interaction):

    prefs = UserPreferences.find_one(discord_user_id=interaction.user.id)
    if not prefs:
        prefs = UserPreferences(discord_user_id=interaction.user.id)

    prefs.set_pings_enabled(True)
    print(f"Set pings enabled to {True} for {interaction.user.id} ({interaction.user})")
    prefs.save_to_database()
    await interaction.response.send_message("Pings enabled!", ephemeral=True)



@config.bot.tree.command(
        name="disable_pings",
        description = "Disables pings from HiveGame.com to be sent to user.",
        guild = discord.Object(id=GUILD_ID)
) 
async def disable_pings(interaction):

    prefs = UserPreferences.find_one(discord_user_id=interaction.user.id)
    if not prefs:
        prefs = UserPreferences(discord_user_id=interaction.user.id)

    prefs.set_pings_enabled(False)
    print(f"Set pings enabled to {False} for {interaction.user.id} ({interaction.user})")
    prefs.save_to_database()
    await interaction.response.send_message("Pings disabled!", ephemeral=True)
