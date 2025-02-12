import discord
import config
import websockets

config.init()

from constants import *
from utils.ext import *
import json

from commands import *


#TODO: better error handling + logging
# but good enough for MVP

@config.bot.event
async def on_ready():
    # Note: commands must sync with Discord's command register
    # it may take up to an hour for commands to be registered
    await config.bot.tree.sync()  
    for guild in config.bot.guilds:
        print(f"Adding commands to {guild.name}")
        await config.bot.tree.sync(guild=guild)


    print("Bot is now Online!")
    asyncio.create_task(handle_message_queue())


async def process_loop(ws):
    while True:
        message = await ws.recv()
        print(f"Received message: {message}")
        try:
            message = json.loads(message) 
        except json.JSONDecodeError as e:
            print("Could not decode message as json, skipping...")
            print(e)
            continue


        if "type" not in message:
            print("Error: Could not retrieve 'type' from message, skipping...")
            return

        if "discord_id" not in message: 
            print("Error: Could not retrieve 'discord_id' from message, skipping...")
            continue
        
        user = config.bot.get_user(int(message["discord_id"]))
        if not user:
            print("Error: Could not retrieve user from discord, skipping...")
            print(f"User ID: {message['discord_id']}")
            continue

        msg = f"<@{user.id}> [PLACEHOLDER TESTING MESSAGE] Your turn on hivegame.com! :D"

        result = False
        if message["type"] == "PING_DM":
            result = await ping_in_dm(user, msg)

        elif message["type"] == "PING_GUILD":
            result = await ping_in_guild(user, msg)

        if result:
            print(f"Successfully pinged user {user.name} in {message['type']}")
        else:
            print(f"Failed to ping user {user.name} in {message['type']}")

async def handle_message_queue(): 
    await reconnecting_websocket(process_loop)


if __name__ == "__main__":
    config.bot.run(BOT_TOKEN)
