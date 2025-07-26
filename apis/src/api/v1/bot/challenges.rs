use crate::api::v1::auth::Auth;
use crate::api::v1::messages::send::{send_challenge_creation_message, send_challenge_messages};
use crate::responses::{ChallengeResponse, GameResponse};
use crate::websocket::{busybee::Busybee, WsServer};
use actix::Addr;
use actix_web::{
    get, post,
    web::{Data, Json, Path},
    HttpResponse,
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Challenge, Game, NewChallenge, NewGame, User},
    DbPool,
};
use hive_lib::{ColorChoice, GameType};
use rand::random;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::{
    ChallengeDetails, ChallengeId, ChallengeVisibility, CorrespondenceMode, TimeMode,
};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum BotTimeControl {
    Untimed,
    RealTime { base: u32, increment: u32 },
    Correspondence { mode: CorrespondenceMode, days: u32 },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotChallengeRequest {
    pub game_type: GameType,
    pub visibility: ChallengeVisibility,
    pub opponent: Option<String>,
    pub color_choice: ColorChoice,
    pub time_control: BotTimeControl,
    pub rated: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}

impl BotChallengeRequest {
    pub fn validate_and_convert(self) -> Result<ChallengeDetails> {
        // Validate game type constraints
        if self.game_type == GameType::Base && self.rated {
            return Err(anyhow::anyhow!("Base game type cannot be rated"));
        }

        if self.game_type != GameType::Base && self.game_type != GameType::MLP {
            return Err(anyhow::anyhow!("Only Base and MLP allowed"));
        }

        // Validate visibility constraints
        if self.visibility == ChallengeVisibility::Direct && self.opponent.is_none() {
            return Err(anyhow::anyhow!(
                "Direct challenges require an opponent username"
            ));
        }

        //Validate rating bands
        if matches!((self.band_lower, self.band_upper), (Some(lower), Some(upper)) if lower > upper)
        {
            return Err(anyhow::anyhow!("Invalid rating restriction"));
        }

        // Validate time control and extract time_mode, time_base, time_increment
        let (time_mode, time_base, time_increment) = match &self.time_control {
            BotTimeControl::Untimed => {
                if self.rated {
                    return Err(anyhow::anyhow!("Untimed games cannot be rated"));
                }
                (TimeMode::Untimed, None, None)
            }
            BotTimeControl::RealTime { base, increment } => {
                if !(1..=180).contains(base) {
                    return Err(anyhow::anyhow!(
                        "RealTime base must be between 1 and 180 minutes"
                    ));
                }
                if *increment > 180 {
                    return Err(anyhow::anyhow!(
                        "RealTime increment must be between 0 and 180 seconds"
                    ));
                }
                (
                    TimeMode::RealTime,
                    Some((*base * 60) as i32),
                    Some(*increment as i32),
                )
            }
            BotTimeControl::Correspondence { mode, days } => {
                if !(1..=20).contains(days) {
                    return Err(anyhow::anyhow!(
                        "Correspondence days must be between 1 and 20"
                    ));
                }
                let days_in_seconds = *days as i32 * 86400;
                match mode {
                    CorrespondenceMode::DaysPerMove => {
                        (TimeMode::Correspondence, None, Some(days_in_seconds))
                    }
                    CorrespondenceMode::TotalTimeEach => {
                        (TimeMode::Correspondence, Some(days_in_seconds), None)
                    }
                }
            }
        };

        // Convert to ChallengeDetails
        Ok(ChallengeDetails {
            rated: self.rated,
            game_type: self.game_type,
            visibility: self.visibility,
            opponent: self.opponent,
            color_choice: self.color_choice,
            time_mode,
            time_base,
            time_increment,
            band_upper: self.band_upper,
            band_lower: self.band_lower,
        })
    }
}

#[get("/api/v1/bot/challenges/")]
pub async fn api_get_challenges(Auth(bot): Auth, pool: Data<DbPool>) -> HttpResponse {
    match get_challenges(bot.id, pool).await {
        Ok(challenges) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
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
    Auth(bot): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    let nanoid = nanoid.into_inner();
    match accept_challenge(nanoid, bot.clone(), pool, ws_server).await {
        Ok(game) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
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

#[post("/api/v1/bot/challenges/")]
pub async fn api_create_challenge(
    Json(req): Json<BotChallengeRequest>,
    Auth(bot): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    let challenge_details = match req.validate_and_convert() {
        Ok(details) => details,
        Err(e) => {
            return HttpResponse::Ok().json(json!({
              "success": false,
              "data": {
                "error": e.to_string(),
              }
            }));
        }
    };

    match create_challenge(challenge_details, bot.id, pool, ws_server).await {
        Ok(challenge) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
            "challenge": challenge,
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

async fn create_challenge(
    req: ChallengeDetails,
    bot_id: Uuid,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<ChallengeResponse> {
    let mut conn = get_conn(&pool).await?;

    let opponent_id = match (&req.visibility, &req.opponent) {
        (ChallengeVisibility::Direct, Some(username)) => {
            Some(User::find_by_username(username, &mut conn).await?.id)
        }
        _ => None,
    };

    let new_challenge = NewChallenge::new(bot_id, opponent_id, &req, &mut conn).await?;
    let challenge = Challenge::create(&new_challenge, &mut conn).await?;
    let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;

    send_challenge_creation_message(ws_server, &challenge_response, &req.visibility, opponent_id)
        .await?;

    Ok(challenge_response)
}

async fn accept_challenge(
    id: ChallengeId,
    bot: User,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<GameResponse> {
    let mut conn = get_conn(&pool).await?;
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
                println!("Failed to send Busybee message: {e}");
            }
        }
    };

    let response = GameResponse::from_model(&game, &mut conn).await?;
    Ok(response)
}

async fn get_challenges(user_id: Uuid, pool: Data<DbPool>) -> Result<Vec<ChallengeResponse>> {
    let mut responses = Vec::new();
    let mut conn = get_conn(&pool).await?;
    let challenges = Challenge::direct_challenges(user_id, &mut conn).await?;
    let own = Challenge::get_own(user_id, &mut conn).await?;
    for challenge in own {
        let response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        responses.push(response);
    }
    for challenge in challenges {
        let response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        responses.push(response);
    }
    Ok(responses)
}
