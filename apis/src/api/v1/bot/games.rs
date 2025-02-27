use crate::api::v1::auth::auth::Auth;
use actix_web::get;
use actix_web::web::{Data, Json};
use actix_web::{post, web, HttpResponse};
use anyhow::{anyhow, Result};
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::State;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;

#[derive(Serialize, Deserialize)]
struct GameRequest {
    selector: GameSelector,
}

#[derive(Serialize, Deserialize)]
pub enum GameSelector {
    Ongoing,
    Pending,
    Specific(GameId),
}

#[post("/api/v1/bot/games")]
pub async fn api_get_games(
    Json(req): Json<GameRequest>,
    Auth(email): Auth,
    pool: Data<DbPool>,
) -> HttpResponse {
    match get_games(&email, req.selector, pool).await {
        Ok(games) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "user": email,
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

async fn get_games(email: &str, selector: GameSelector, pool: Data<DbPool>) -> Result<Vec<Game>> {
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_email(email, &mut conn).await?;
    Ok(match selector {
        GameSelector::Ongoing => user.get_ongoing_games(&mut conn).await?,
        GameSelector::Specific(id) => {
            [Game::find_by_game_id(&id, &mut conn).await?].to_vec()
        }
        GameSelector::Pending => user.get_games_with_notifications(&mut conn).await?,
    })
}
