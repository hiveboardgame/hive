# Hive Hydra

A client/server wrapper for multiple Bot accounts on hivegame.com to and multiple UHP AI engines.


## Notes about Hive Bot API


Login as a bot

post {"email": "bot@example.com", "password": "hivegame"} 127.0.0.1:3000/api/v1/auth/token to get a jwt token
and use the token as Bearer to do the following things:

curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"email": "bot@example.com", "password": "hivegame"}' \
  http://127.0.0.1:3000/api/v1/auth/token

Response: {"data":{"token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJhbmRyZWEuZnJpZ2lkbytib3RAZ21haWwuY29tIiwiaXNzIjoiaGl2ZWdhbWUuY29tIiwiZXhwIjoxNzQzMDMyMzI0fQ.oX7zJLRXe-cEXEdiyAF3Jk1_3OOOQdriKHYpj-CTt6o"},"success":true}


get 127.0.0.1:3000/api/v1/auth/id just to see that your token is valid


Get games waiting for a move

get /api/v1/bot/games/pending


post {"selector":{"Specific":"iLGbFmm7C9Xn"}} or {"selector":"Ongoing"} or {"selector":"Pending"} 127.0.0.1:3000/api/v1/bot/games to get games



post {"game_id": "iLGbFmm7C9Xn", "piece": "bQ", "position": "bL-"} 127.0.0.1:3000/api/v1/bot/play to play a turn


Get challenges from other players

get /api/v1/bot/challenges


