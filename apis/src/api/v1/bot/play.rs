use crate::api::v1::auth::auth::Auth;
use crate::websocket::ClientActorMessage;
use codee::binary::MsgpackSerdeCodec;
use crate::common::ServerResult;
use crate::websocket::WsServer;
use actix::Addr;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse};
use anyhow::{anyhow, Result};
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::{Piece, Position, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;
use std::str::FromStr;
use crate::api::v1::messages::send::send_messages;

#[derive(Serialize, Deserialize)]
struct PlayRequest {
    game_id: GameId,
    piece_pos: String,
}

#[post("/api/v1/bot/games/play")]
pub async fn api_play(
    Json(req): Json<PlayRequest>,
    Auth(email): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    match play_move(req, &email, pool, ws_server).await {
        Ok((game, _turn)) => {
            HttpResponse::Ok().json(json!({
              "success": true,
              "data": {
                "bot": email,
                "history": game.history,
              }
            }))
        }
        Err(e) => HttpResponse::Ok().json(json!({
          "success": false,
          "data": {
            "error": e.to_string(),
          }
        })),
    }
}

async fn play_move(play: PlayRequest, email: &str, pool: Data<DbPool>, ws_server: Data<Addr<WsServer>>) -> Result<(Game, Turn)> {
    if let Some((piece_str, pos_str)) = play.piece_pos.split_once(' ') {
        let piece = Piece::from_str(piece_str)?;
        let cloned_pool = pool.clone();
        let mut conn = get_conn(&cloned_pool).await?;
        let game = Game::find_by_game_id(&play.game_id, &mut conn).await?;
        let user = User::find_by_email(email, &mut conn).await?;
        if game.current_player_id != user.id {
            return Err(anyhow!("Not your turn"));
        }
        let mut state = State::new_from_str(&game.history, &game.game_type)?;
        let position = if state.turn == 0 {
            Position::initial_spawn_position()
        } else {
            Position::from_string(pos_str, &state.board)?
        };

        let played_turn = Turn::Move(piece, position);

        let (game, played_turn_out) = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    state.play_turn_from_position(piece, position)?;
                    let updated_game = game.update_gamestate(&state, 0_f64, tc).await?;
                    send_messages(ws_server.clone(), &updated_game, &user, &pool, played_turn.clone()).await?;
                    Ok((updated_game, played_turn))
                 }.scope_boxed()
            })
            .await?;
        return Ok((game, played_turn_out));
    } else {
        return Err(anyhow!("Move is not correct"));
    }
}
