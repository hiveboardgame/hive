use crate::api::v1::auth::auth::Auth;
use crate::responses::{ChallengeResponse, GameResponse};
use crate::websocket::WsServer;
use crate::api::v1::messages::send::send_challenge_messages;
use crate::websocket::busybee::Busybee;
use actix::Addr;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Challenge, Game, NewGame, User},
    DbPool,
};
use serde_json::json;
use shared_types::{ChallengeId, TimeMode};
use rand::random;
use std::str::FromStr;

#[get("/api/v1/bot/challenges/")]
pub async fn api_get_challenges(Auth(email): Auth, pool: Data<DbPool>) -> HttpResponse {
    match get_challenges(&email, pool).await {
        Ok(challenges) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": email,
            "challenges": challenges,
          }
        })),
        Err(e) => HttpResponse::Ok().json(json!({
          "success": false,
          "data": {
            "error": e.to_string(),
          }
        })),
    }
}

#[get("/api/v1/bot/challenge/accept/{nanoid}")]
pub async fn api_accept_challenge(
    nanoid: Path<ChallengeId>,
    Auth(email): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    let nanoid = nanoid.into_inner();
    match accept_challenge(nanoid, &email, pool, ws_server).await {
        Ok(game) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": email,
            "game": game,
          }
        })),
        Err(e) => HttpResponse::Ok().json(json!({
          "success": false,
          "data": {
            "error": e.to_string(),
          }
        })),
    }
}

async fn accept_challenge(
    id: ChallengeId,
    email: &str,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<GameResponse> {
    let mut conn = get_conn(&pool).await?;
    let bot = User::find_by_email(email, &mut conn).await?;
    let challenge = Challenge::find_by_challenge_id(&id, &mut conn).await?;
    let (white_id, black_id) = match challenge.color_choice.to_lowercase().as_str() {
        "black" => (bot.id, challenge.challenger_id),
        "white" => (challenge.challenger_id, bot.id),
        _ => {
            if random() {
                (challenge.challenger_id, bot.id)
            } else {
                (bot.id, challenge.challenger_id)
            }
        }
    };
    let new_game = NewGame::new(white_id, black_id, &challenge);
    let (game, deleted_challenges) =
        Game::create_and_delete_challenges(new_game, &mut conn).await?;
    
    send_challenge_messages(ws_server, deleted_challenges, &game, &bot, &pool).await?;
    
    match TimeMode::from_str(&game.time_mode) {
        Ok(TimeMode::RealTime) | Err(_) => {}
        _ => {
            let challenger_id = challenge.challenger_id;
            let msg = format!(
                "[Game started](<https://hivegame.com/game/{}>) - Your game with {} has started.\nYou have {} to play.",
                game.nanoid,
                bot.username,
                game.str_time_left_for_player(challenger_id)
            );

            if let Err(e) = Busybee::msg(challenger_id, msg).await {
                println!("Failed to send Busybee message: {}", e);
            }
        }
    };
    
    let response = GameResponse::from_model(&game, &mut conn).await?;
    Ok(response)
}

async fn get_challenges(email: &str, pool: Data<DbPool>) -> Result<Vec<ChallengeResponse>> {
    let mut responses = Vec::new();
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_email(email, &mut conn).await?;
    let challenges = Challenge::direct_challenges(user.id, &mut conn).await?;
    for challenge in challenges {
        let response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        responses.push(response);
    }
    Ok(responses)
}
