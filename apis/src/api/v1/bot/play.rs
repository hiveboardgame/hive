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
struct PlayRequest {
    game_id: GameId,
    piece: String,
    position: String,
}

#[post("/api/v1/bot/play")]
pub async fn api_play(
    Json(req): Json<PlayRequest>,
    Auth(email): Auth,
    pool: Data<DbPool>,
) -> HttpResponse {
    match play_move(req, &email, pool).await {
        Ok(game) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "user": email,
            "history": game.history,
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

async fn play_move(play: PlayRequest, email: &str, pool: Data<DbPool>) -> Result<Game> {
    let mut conn = get_conn(&pool).await?;
    let game = Game::find_by_game_id(&play.game_id, &mut conn).await?;
    let user = User::find_by_email(email, &mut conn).await?;
    if game.current_player_id != user.id {
        return Err(anyhow!("Not your turn"));
    }
    let mut state = State::new_from_str(&game.history, &game.game_type)?;
    state.play_turn_from_history(&play.piece, &play.position)?;
    let game = conn
        .transaction::<_, anyhow::Error, _>(move |tc| {
            async move { Ok(game.update_gamestate(&state, 0_f64, tc).await?) }.scope_boxed()
        })
        .await?;
    // TODO: inform user via busybee and via websocket message
    Ok(game)
}
