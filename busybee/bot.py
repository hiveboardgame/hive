import discord
import config
import websockets
import logging

config.init()

from constants import *
from utils.ext import *
import json

from commands import *

logger = logging.getLogger("bot")


# TODO: better error handling + logging
# but good enough for MVP


@config.bot.event
async def on_ready():
    # Note: commands must sync with Discord's command register
    # it may take up to an hour for commands to be registered
    logger.info("Syncing global commands with Discord's servers...")
    await config.bot.tree.sync()
    logger.info("Global commands synced!")
    for guild in config.bot.guilds:
        logger.info(f"Adding commands to server: '{guild.name}'...")
        await config.bot.tree.sync(guild=guild)

    logger.info("Bot is now Online!")
    asyncio.create_task(handle_message_queue())


async def process_loop(ws):
    while True:
        message = await ws.recv()
        logger.debug(f"Received message: {message}")
        try:
            message = json.loads(message)
        except json.JSONDecodeError as e:
            logger.error(f"Could not decode message as json, skipping... {e}")
            continue

        if "type" not in message:
            logger.error("Could not retrieve 'type' from message, skipping...")
            return

        if "discord_id" not in message:
            logger.error("Could not retrieve 'discord_id' from message, skipping...")
            continue

        user = config.bot.get_user(int(message["discord_id"]))
        if not user:
            logger.error(
                "Could not retrieve user from discord, skipping...\n"
                f"User ID: {message['discord_id']}"
            )
            continue

        msg = (
            f"<@{user.id}> [PLACEHOLDER TESTING MESSAGE] Your turn on hivegame.com! :D"
        )
        if message["type"] == "MSG_DM": 
            msg = message["content"]
        if message["type"] == "MSG_GUILD":
            content = message["content"]
            msg = f"<@{user.id}> {content}"


        result = False
        if message["type"] == "PING_DM" or message["type"] == "MSG_DM":
            result = await ping_in_dm(user, msg)

        elif message["type"] == "PING_GUILD" or message["type"] == "MSG_GUILD":
            result = await ping_in_guild(user, msg)

        if result:
            logger.info(f"Successfully pinged user {user.name} in {message['type']}")
        else:
            logger.error(f"Failed to ping user {user.name} in {message['type']}")


async def handle_message_queue():
    await reconnecting_websocket(process_loop)


if __name__ == "__main__":
    config.bot.run(BOT_TOKEN)
