# Hive Discord Bot MVP

This is a proof of concept for a discord bot that does the following:

- Connects a provided uuid to a discord user via oauth
- Pings a user in discord whenever a certain endpoint is hit
- Allows users on discord to disable pings via discord commands

## Launching 

To launch the bot needs a few environment variables set:

```
# These can be found in the discord developer portal
DISCORD_BOT_TOKEN # The token for the discord bot
DISCORD_CLIENT_ID # The client id for the discord bot
DISCORD_CLIENT_SECRET # The client secret for the discord bot

# This must be provisioned separately
DISCORD_BOT_DATABASE_URL # The sqlite database url for the bot
```

Once these are set you can run the bot with the following commands (be sure to set up an python3 virtual environment first):

Install the requirements:

```
python3 -r requirements.txt
```

Run the server
```
uvicorn api:app --host 0.0.0.0 --port 8080
```

Run the Discord Bot
```
python3 bot.py 
```

## TODOS

I'm planning to clean up the code a bit and add some more features. Here are some of the things I'm planning to do:

- [ ] Make PORT and REDIRECT\_URI configurable
- [ ] Documentation for API endpoints fleshed out
- [ ] Containerize everything and add Nix flack command to have single command setup
- [ ] Custom Message endpoint
- [ ] Clean up requirements file
- [ ] Clean up dead code
- [ ] Add structured logging

## Endpoints

```
// Returns a new discord.com hosted url for initiating the Oauth flow. 
// Returns error message and error code if unsuccessful 
// ( configuration incorrect, missing discord token etc ...)
//
POST localhost:8080/oauth/new/{hive_game_user_id}

// uses the callback code and state URL parameters from discord provided 
// at the end of the Oauth to verify successful integration, 
// if code checks out, link hive_game_user_id <--> discord_user_id
// all pings should work now
//
POST localhost:8080/oauth/callback?code={CALLBACK_CODE}&state={CALLBACK_STATE}

// pings the corresponding discord user saying that it is their turn to move on hivegame
// returns error message and if ping was unsuccessful 
// (invalid user, user left server, dm not reachable etc..)
//
POST localhost:8080/ping/{hive_game_user_id } 


// Gets the ID of the discord user linked to the hive game user
// returns error message if unsuccessful
//
GET localhost:8080/discord_id/{hive_game_user_id}
```
