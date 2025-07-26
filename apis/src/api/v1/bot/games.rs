use crate::api::v1::auth::Auth;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;

#[derive(Serialize, Deserialize)]
pub enum GameSelector {
    Ongoing,
    Pending,
    Specific(GameId),
}

#[get("/api/v1/bot/game/{nanoid}")]
pub async fn api_get_game(
    nanoid: Path<GameId>,
    Auth(bot): Auth,
    pool: Data<DbPool>,
) -> HttpResponse {
    let nanoid = nanoid.into_inner();
    match get_games(bot.clone(), GameSelector::Specific(nanoid), pool).await {
        Ok(games) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
            "games": games,
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

#[get("/api/v1/bot/games/ongoing")]
pub async fn api_get_ongoing_games(Auth(bot): Auth, pool: Data<DbPool>) -> HttpResponse {
    match get_games(bot.clone(), GameSelector::Ongoing, pool).await {
        Ok(games) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
            "games": games,
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

#[get("/api/v1/bot/games/pending")]
pub async fn api_get_pending_games(Auth(bot): Auth, pool: Data<DbPool>) -> HttpResponse {
    match get_games(bot.clone(), GameSelector::Pending, pool).await {
        Ok(games) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
            "games": games,
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

async fn get_games(bot: User, selector: GameSelector, pool: Data<DbPool>) -> Result<Vec<Game>> {
    let mut conn = get_conn(&pool).await?;
    Ok(match selector {
        GameSelector::Ongoing => bot.get_ongoing_games(&mut conn).await?,
        GameSelector::Specific(id) => [Game::find_by_game_id(&id, &mut conn).await?].to_vec(),
        GameSelector::Pending => bot.get_games_with_notifications(&mut conn).await?,
    })
}
