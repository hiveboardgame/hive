from discord.ext import commands
from discord import app_commands
import discord
from utils import pretty_print

import sys
import traceback
import config

config.init()

from models import *
from constants import GUILD_ID, BOT_NAME

import logging

logger = logging.getLogger(__name__)


async def send_help(interaction):
    embed = discord.Embed(
        title="Help Menu",
        description="Here's how you can use the bot:",
        color=discord.Color.blue(),
    )
    embed.add_field(
        name="`/pings status`",
        value="Check to see if pings are enabled for user",
        inline=False,
    )
    embed.add_field(
        name="`/pings enable`",
        value="Enable pings from HiveGame.com to be sent to user",
        inline=False,
    )
    embed.add_field(
        name="`/pings disable`",
        value="Disables pings from HiveGame.com to be sent to user",
        inline=False,
    )
    await interaction.response.send_message(embed=embed)


@config.bot.tree.command(
    name="help",
    description="Get help on how to use {BOT_NAME}",
    guild=discord.Object(id=GUILD_ID),
)
async def help(interaction):
    await send_help(interaction)


@config.bot.tree.command(
    name="info",
    description="Get help on how to use {BOT_NAME}!",
    guild=discord.Object(id=GUILD_ID),
)
async def help(interaction):
    await send_help(interaction)


ping_group = app_commands.Group(
    name="pings", description="Commands for managing pings from HiveGame.com"
)
config.bot.tree.add_command(ping_group)


@ping_group.command(
    name="status",
    description="Check to see if pings are enabled for user",
)
async def status(interaction):
    user_record = UserRecord.find_one(discord_user_id=interaction.user.id)
    linked_on_hive = user_record is not None
    prefs = UserPreferences.find_one(discord_user_id=interaction.user.id)
    pings_enabled = prefs.pings_enabled() if prefs else False

    await interaction.response.send_message(
        f"Pings are {'enabled' if pings_enabled else 'disabled'} for {interaction.user}. \n"
        f"This discord account is {'linked' if linked_on_hive else 'not yet linked'} to a Hivegame account.",
    )


@ping_group.command(
    name="enable",
    description="Enable pings from HiveGame.com to be sent to user.",
)
async def enable_pings(interaction):

    prefs = UserPreferences.find_one(discord_user_id=interaction.user.id)
    if not prefs:
        await interaction.response.send_message(
            "User not linked to any Hivegame account! Please link your account first."
        )
        return

    prefs.set_pings_enabled(True)
    logger.info(
        f"Set pings enabled to {True} for {interaction.user.id} ({interaction.user})"
    )
    prefs.save_to_database()
    await interaction.response.send_message("Pings enabled!")


@ping_group.command(
    name="disable",
    description="Disables pings from HiveGame.com to be sent to user.",
)
async def disable_pings(interaction):

    prefs = UserPreferences.find_one(discord_user_id=interaction.user.id)
    if not prefs:
        await interaction.response.send_message(
            "User not linked Hivegame account! Please link your account first."
        )
        return

    prefs.set_pings_enabled(False)
    logger.info(
        f"Set pings enabled to {False} for {interaction.user.id} ({interaction.user})"
    )
    prefs.save_to_database()
    await interaction.response.send_message("Pings disabled!")
