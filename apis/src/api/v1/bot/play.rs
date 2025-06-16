use crate::api::v1::auth::auth::Auth;
use crate::api::v1::messages::send::send_messages;
use crate::common::ServerResult;
use crate::websocket::ClientActorMessage;
use crate::websocket::WsServer;
use actix::Addr;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse};
use anyhow::{anyhow, Result};
use codee::binary::MsgpackSerdeCodec;
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
use shared_types::{GameId, TimeMode};
use std::str::FromStr;
use crate::websocket::busybee::Busybee;

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
        Ok((game, _turn)) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": email,
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

async fn play_move(
    play: PlayRequest,
    email: &str,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<(Game, Turn)> {
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&play.game_id, &mut conn).await?;
    let bot = User::find_by_email(email, &mut conn).await?;
    if game.current_player_id != bot.id {
        return Err(anyhow!("Not your turn"));
    }
    let mut state = State::new_from_str(&game.history, &game.game_type)?;

    let (piece, position) = if state.turn == 0 {
        let piece = Piece::from_str(&play.piece_pos)?;
        let position = Position::initial_spawn_position();
        (piece, position)
    } else {
        let (piece_str, pos_str) = play.piece_pos
            .split_once(' ')
            .ok_or_else(|| anyhow!("Invalid move format: expected 'piece position'"))?;
        
        let piece = Piece::from_str(piece_str)?;
        let position = Position::from_string(pos_str, &state.board)?;
        (piece, position)
    };

    let played_turn = Turn::Move(piece, position);

    let (game, played_turn_out) = conn
        .transaction::<_, anyhow::Error, _>(move |tc| {
            async move {
                state.play_turn_from_position(piece, position)?;
                let updated_game = game.update_gamestate(&state, 0_f64, tc).await?;
                send_messages(
                    ws_server.clone(),
                    &updated_game,
                    &bot,
                    &pool,
                    played_turn.clone(),
                )
                .await?;

                match TimeMode::from_str(&updated_game.time_mode) {
                    Ok(TimeMode::RealTime) | Err(_) => {}
                    _ => {
                        let opponent_id = updated_game.current_player_id;
                        let msg = format!(
                            "[Your turn](<https://hivegame.com/game/{}>) in your game vs {}.\nYou have {} to play.",
                            updated_game.nanoid,
                            bot.username,
                            updated_game.str_time_left_for_player(opponent_id)
                        );

                        if let Err(e) = Busybee::msg(opponent_id, msg).await {
                            println!("Failed to send Busybee message: {}", e);
                        }
                    }
                };

                Ok((updated_game, played_turn))
            }
            .scope_boxed()
        })
        .await?;
    Ok((game, played_turn_out))
}
