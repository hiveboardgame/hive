import discord
from discord.ext import commands
import asyncio

import functools

import requests 

import sys
import os

import errors
import constants
import time
from websockets.asyncio.client import connect

def output_to_channel(*channels):
    def inner(cog_function):
        @functools.wraps(cog_function)
        async def wrapper(cls, ctx, *args, **kwargs):

            output_channels = []
            for channel in cls.bot.get_all_channels():
                if str(channel.type) == "text":
                    if channel.name.lower() in [i.lower() for i in channels]:
                        output_channels.append(channel)
                    elif str(channel.id) in channels:
                        output_channels.append(channel)
            ctx.output_channels = output_channels

            await cog_function(cls, ctx, *args, **kwargs)

        return wrapper

    return inner


def dm_only(cog_function):
    @functools.wraps(cog_function)
    async def wrapper(cls, ctx, *args, **kwargs):
        if ctx.guild:
            raise errors.PrivateMessageOnly("This has personal information in it!")
        return await cog_function(cls, ctx, *args, **kwargs)

    return wrapper


def send_to_dm(cog_function):
    @functools.wraps(cog_function)
    async def wrapper(cls, ctx, *args, **kwargs):
        ctx.channel = await ctx.author.create_dm()
        return await cog_function(cls, ctx, *args, **kwargs)

    return wrapper

async def ping_in_dm(user, msg) -> bool:
    dm = await user.create_dm()
    try:
        await dm.send(msg)
    except discord.Forbidden as e:
        print("Could not send message to user")
        print(e)
        return False

    return True

async def ping_in_guild(user, msg):
    choices = []
    for guild in user.mutual_guilds:
        channels = guild.text_channels
        allowed_channels = [c for c in channels if c.id in PING_CHANNELS_IDS]
        choices.extend(allowed_channels)

    if not choices:
        print("Bot cache did not contain valid channels to ping user in")
        print("Are the PING_CHANNELS_IDS correctly configured?")
        return False

    choices.sort(key=lambda x: PING_CHANNELS_IDS.index(x.id)) 

    for channel in choices:
        if not channel.permissions_for(guild.me).send_messages:
            print(f"Bot does not have permission to send messages in channel {channel.name} in guild {channel.guild.name}, skipping...")
            continue

        try:
            await channel.send(msg)
            return True
        except discord.Forbidden as e:
            print(f"Could not send message to user in channel {channel.name} in guild {channel.guild.name} due to the following error:")
            print(e)
            print("Trying next channel...")
            continue

    print("Exhausted all channels, could not send message to user in any channel")
    print("Are the PING_CHANNELS_IDS correctly configured?")
    return False

async def reconnecting_websocket(process_func):
    print("Connecting to message queue websocket...")
    async for ws in connect(constants.WS_URL, ping_timeout=None):
        print("Connected to message queue websocket!")

        try:
            await ws.send("hello")
            data = await ws.recv()
            if data != "hello": ws.close()
            await process_func(ws)
        except Exception as e:
            print("Connection to message queue websocket was severed...")
            print(f"Error: {e}")
            print(f"Reconnecting in {constants.RETRY_TIMEOUT_SECONDS} seconds...")
            await asyncio.sleep(constants.RETRY_TIMEOUT_SECONDS)

