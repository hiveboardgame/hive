import config

config.init()
import logging

from pydantic import BaseModel
from constants import *
from asyncio import Queue

from fastapi import FastAPI, Request, HTTPException, WebSocket
from fastapi.responses import JSONResponse
from models import *
from utils.ext import *

from rauth import OAuth2Service

logger = logging.getLogger("api")

MAX_QUEUE_SIZE = 3

app = FastAPI()
message_queue = Queue(maxsize=MAX_QUEUE_SIZE)

discord_oauth = OAuth2Service(
    name="discord",
    client_id=DISCORD_CLIENT_ID,
    client_secret=DISCORD_CLIENT_SECRET,
    access_token_url="https://discord.com/api/oauth2/token",
    authorize_url=f"https://discord.com/api/oauth2/authorize",
    base_url="https://discord.com/api/",
)


def init_user(discord_id, hive_user_id):
    new_user = UserRecord(discord_user_id=discord_id, hive_user_id=hive_user_id)
    new_user_preferences = UserPreferences.find_one(
        discord_user_id=discord_id
    ) or UserPreferences(discord_user_id=discord_id)
    new_user_preferences.save_to_database()
    new_user.save_to_database()
    return new_user


# A quick and dirty test to see if the discord bot is operational,
# spam the message queue with a bunch of hello messages and
# see if the discord bot consumes the message queue quickly
async def queue_empties_quickly() -> bool:
    try:
        async with asyncio.timeout(1):
            hello_msg = json.dumps({"type": "HELLO"})
            for i in range(MAX_QUEUE_SIZE + 1):
                await message_queue.put(hello_msg)
    except asyncio.TimeoutError:
        return False
    return True


@app.get("/health")
async def health(request: Request):
    if not await queue_empties_quickly():
        raise HTTPException(
            500,
            detail="Message queue is not emptying quickly enough, the discord bot service may be down or disconnected from the message stream.",
        )

    try:
        UserRecord.find_one(discord_user_id=0)  # Test database connection
    except Exception as e:
        raise HTTPException(500, detail="Database connection error")

    return JSONResponse({"detail": "alive"})


@app.post("/oauth/new/{hive_user_id}")
async def start_flow(request: Request, hive_user_id: str):

    token = OauthState.generate_token(hive_user_id)

    auth_url = discord_oauth.get_authorize_url(
        redirect_uri=REDIRECT_URI, state=token, response_type="code", scope="identify"
    )

    return JSONResponse({"url": auth_url})


@app.post("/oauth/callback")
async def end_flow(request: Request, code: str, state: str):

    access_token = None
    try:
        access_token = discord_oauth.get_access_token(
            data={
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": REDIRECT_URI,
            },
            decoder=json.loads,
        )
    except Exception as e:
        raise HTTPException(
            401,
            detail="Discord code is likely invalid or this is a repeat request (Codes are one time use only).",
        )

    authorized_session = discord_oauth.get_session(access_token)
    response = authorized_session.get("users/@me").json()
    discord_id = response.get("id")
    discord_id = int(discord_id)
    username = response.get("username")
    avatar_hash = response.get("avatar")

    if avatar_hash:
        avatar_url = f"https://cdn.discordapp.com/avatars/{discord_id}/{avatar_hash}.png"
    else:
        avatar_url = None

    if not discord_id:
        raise HTTPException(401, detail="Cannot retrieve discord id")

    if not OauthState.is_valid(state):
        raise HTTPException(401, detail="Invalid oauth secret")

    hive_id = OauthState.find_one(token=state).hive_user_id

    # We no longer need to hold the state in the database,
    # delete to prevent replay attacks
    OauthState.delete_from_database(hive_user_id=hive_id)

    user = init_user(discord_id, hive_id)
    if user:
        user.avatar_url = avatar_url
        user.username = username
        user.save_to_database()
        return JSONResponse({"detail": "User linked successfully"})
    else:
        raise HTTPException(500, detail="User link failed")


class Message(BaseModel):
    content: str


@app.post("/msg/{hive_user_id}")
async def msg_user(request: Request, hive_user_id: str, message: Message):
    user = UserRecord.find_one(hive_user_id=hive_user_id)

    if not user:
        raise HTTPException(404, detail="User not linked yet")
    discord_id = user.discord_user_id

    user_prefs = UserPreferences.find_one(discord_user_id=discord_id)
    if not user_prefs:
        raise HTTPException(404, detail="User preferences not found")

    if not user_prefs.pings_enabled():
        raise HTTPException(
            400, detail="User ping unsuccessful, user has pings disabled"
        )

    msg = dict()
    msg["discord_id"] = discord_id
    msg["content"] = message.content

    success_response = None

    if user_prefs.send_pings_to_dm_enabled():
        success_response = JSONResponse({"detail": "Ping message for DM sent to queue"})
        msg["type"] = "MSG_DM"
    else:
        success_response = JSONResponse(
            {"detail": "Ping message for Guild sent to queue"}
        )
        msg["type"] = "MSG_GUILD"

    try:
        await message_queue.put(json.dumps(msg))
    except Exception as e:
        raise HTTPException(
            500,
            detail="Message queue is full! Is the discord bot running and connected?",
        )

    return success_response


@app.post("/ping/{hive_user_id}")
async def ping_user(request: Request, hive_user_id: str):
    user = UserRecord.find_one(hive_user_id=hive_user_id)

    if not user:
        raise HTTPException(404, detail="User not linked yet")
    discord_id = user.discord_user_id

    user_prefs = UserPreferences.find_one(discord_user_id=discord_id)
    if not user_prefs:
        raise HTTPException(404, detail="User preferences not found")

    if not user_prefs.pings_enabled():
        raise HTTPException(
            400, detail="User ping unsuccessful, user has pings disabled"
        )

    msg = {"discord_id": discord_id}
    success_response = None

    if user_prefs.send_pings_to_dm_enabled():
        success_response = JSONResponse({"detail": "Ping message for DM sent to queue"})
        msg["type"] = "PING_DM"
    else:
        success_response = JSONResponse(
            {"detail": "Ping message for Guild sent to queue"}
        )
        msg["type"] = "PING_GUILD"

    try:
        await message_queue.put(json.dumps(msg))
    except Exception as e:
        raise HTTPException(
            500,
            detail="Message queue is full! Is the discord bot running and connected?",
        )

    return success_response


@app.get("/discord/{hive_user_id}")
async def discord_id(request: Request, hive_user_id: str):
    user = UserRecord.find_one(hive_user_id=hive_user_id)
    if not user:
        raise HTTPException(404, detail="User not linked yet")

    return JSONResponse({
        "discord_id": user.discord_user_id,
        "avatar_url": user.avatar_url,
        "username": user.username
    })


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()

    data = await websocket.receive_text()
    if data != "hello":
        websocket.close()
    await websocket.send_text(f"hello")

    while True:
        message = await message_queue.get()
        logger.info(f"Sending message to discord bot: {message}")
        await websocket.send_text(message)
