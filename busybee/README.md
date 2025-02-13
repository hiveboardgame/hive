# Busybee - Hivegame's Discord Bot Service

Busybee is a Discord Bot service that enables the following:

- Connecting a provided uuid from Hivegame to a Discord user via Oauth2
- Messaging a ping to a user in Discord whenever a certain endpoint is called
- Allowing users on Discord to manage ping settings via Discord user commands

Busybee exposes a locally hosted API for the Hivegame's core to interact with.
This API allows for interfacing with the Discord Community that Busybee is apart of.

## Launching Manually  

### Prerequisites

To launch, the service looks for the following environment variables

```
# These can be found in the Discord developer portal
DISCORD_BOT_TOKEN # The token for the Discord Bot
DISCORD_CLIENT_ID # The client id for the Discord Bot
DISCORD_CLIENT_SECRET # The client secret for the Discord Bot

# Note: new redirect uris must be added to the Discord Developer Portal
BUSYBEE_API_REDIRECT_URI # The redirect uri for the Oauth2 flow (ex. https://hivegame.com/oauth/callback)
```

Setup a new Python virtual environment and install the requirements:

```console
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

### Launching

There are two processes that need to be launched, the API and the Discord Bot:

```console
source venv/bin/activate                              # If not already activated
uvicorn api:app --host 0.0.0.0 --port 8080&           # Launches the API server
python3 bot.py                                        # Launches the Discord Bot
```

## Launching with Docker

```console
docker build -t busybee .
docker run -d -p 8080:8080 --env-file PATH_TO_DOT_ENV -v PATH_TO_DB_FILE:/code/busybee.db busybee
```

Replace `PATH_TO_DOT_ENV` with the location of your .env and `PATH_TO_DB_FILE` with the location of the sqlite database.

## TODOS

Planning to clean up the code a bit and add some more features:

- [x] Make REDIRECT\_URI configurable
- [x] Add `/info` and `/help` user commands
- [ ] Documentation for API endpoints fleshed out 
    - [ ] document expected responses 
    - [ ] add examples
- [x] Containerize and have 1-line docker launch script 
- [ ] Custom Message endpoint
- [x] Add better logging
- [ ] Clean up requirements file
- [ ] Clean up dead code
- [ ] Make port configurable?

## Endpoints

```
// Health check endpoint
//
// Returns 200 if the API is alive, connected
// to the database, and the Discord Bot is in operation. 
//
// Returns an error code with an explanation if issues are found
// 
GET localhost:8080/health

// Returns a new discord.com hosted url for initiating the Oauth2 flow. 
//
// Returns error code with an explanation if unsuccessful 
//
POST localhost:8080/oauth/new/{HIVE_GAME_USER_ID}

// Accepts the code and state URL parameters from the discord callback provided 
// at the end of the Oauth2 flow. If the code and state is valid, 
// links the provided HIVE_GAME_USER_ID to a DISCORD_USER_ID extracted from the 
// code and state in Busybee's database.
// 
// Returns error code with an explanation if unsuccessful
//
POST localhost:8080/oauth/callback?code={CALLBACK_CODE}&state={CALLBACK_STATE}

// Pings the corresponding Discord user with a standard message 
// saying that it is their turn to move on hivegame
//
// Returns error code with explanation if ping was unsuccessful 
//
POST localhost:8080/ping/{HIVE_GAME_USER_ID} 

// Gets the information of a Discord user that is linked to the provided 
// HIVE_GAME_USER_ID 
//
// Returns error code with an explanation if unsuccessful
//
GET localhost:8080/discord/{HIVE_GAME_USER_ID}
```
