use crate::api::v1::auth::Auth;
use crate::api::v1::messages::send::{send_control_messages, send_turn_messages};
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
use hive_lib::{Color, GameControl, Piece, Position, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
struct PlayRequest {
    game_id: GameId,
    piece_pos: String,
}

#[derive(Serialize, Deserialize)]
struct ControlRequest {
    game_id: GameId,
    control: String,
}

#[post("/api/v1/bot/games/play")]
pub async fn api_play(
    Json(req): Json<PlayRequest>,
    Auth(bot): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    match play_move(req, bot.clone(), pool, ws_server).await {
        Ok((game, _turn)) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
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
    bot: User,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<(Game, Turn)> {
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&play.game_id, &mut conn).await?;
    if game.finished {
        return Err(anyhow!("Game is finished"));
    }
    if game.current_player_id != bot.id {
        return Err(anyhow!("Not your turn"));
    }
    let mut state = State::new_from_str(&game.history, &game.game_type)?;

    let (piece, position) = if state.turn == 0 {
        let piece = Piece::from_str(&play.piece_pos)?;
        let position = Position::initial_spawn_position();
        (piece, position)
    } else {
        let (piece_str, pos_str) = play
            .piece_pos
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
                send_turn_messages(
                    ws_server.clone(),
                    &updated_game,
                    &bot,
                    &pool,
                    played_turn.clone(),
                )
                .await?;

                // Disabled because it spams too much
                //
                // use crate::websocket::busybee::Busybee;
                // match TimeMode::from_str(&updated_game.time_mode) {
                //     Ok(TimeMode::RealTime) | Err(_) => {}
                //     _ => {
                //         let opponent_id = updated_game.current_player_id;
                //         let msg = format!(
                //             "[Your turn](<https://hivegame.com/game/{}>) in your game vs {}.\nYou have {} to play.",
                //             updated_game.nanoid,
                //             bot.username,
                //             updated_game.str_time_left_for_player(opponent_id)
                //         );

                //         if let Err(e) = Busybee::msg(opponent_id, msg).await {
                //             println!("Failed to send Busybee message: {}", e);
                //         }
                //     }
                // };

                Ok((updated_game, played_turn))
            }
            .scope_boxed()
        })
        .await?;
    Ok((game, played_turn_out))
}

#[post("/api/v1/bot/games/control")]
pub async fn api_control(
    Json(req): Json<ControlRequest>,
    Auth(bot): Auth,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> HttpResponse {
    match handle_control(req, bot.clone(), pool, ws_server).await {
        Ok(game) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": bot.email,
            "bot_username": bot.username,
            "game_id": game.nanoid,
            "finished": game.finished,
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

async fn handle_control(
    req: ControlRequest,
    bot: User,
    pool: Data<DbPool>,
    ws_server: Data<Addr<WsServer>>,
) -> Result<Game> {
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&req.game_id, &mut conn).await?;

    if game.finished {
        return Err(anyhow!("Game is finished"));
    }

    let bot_color = if game.white_id == bot.id {
        Color::White
    } else if game.black_id == bot.id {
        Color::Black
    } else {
        return Err(anyhow!("Not your game"));
    };

    let game_control = match req.control.as_str() {
        "resign" => {
            if game.turn < 2 {
                return Err(anyhow!("Cannot resign before turn 2"));
            }
            GameControl::Resign(bot_color)
        }
        "abort" => {
            if game.turn >= 2 {
                return Err(anyhow!("Cannot abort after turn 2"));
            }
            if game.tournament_id.is_some() {
                return Err(anyhow!("Cannot abort tournament games"));
            }
            GameControl::Abort(bot_color)
        }
        _ => return Err(anyhow!("Invalid control type: {}", req.control)),
    };

    if let Some(last_control) = game.last_game_control() {
        if last_control == game_control {
            return Err(anyhow!("Control already sent"));
        }
    }

    let updated_game = conn
        .transaction::<_, anyhow::Error, _>(move |tc| {
            async move {
                let result_game = match game_control {
                    GameControl::Resign(_) => game.resign(&game_control, tc).await?,
                    GameControl::Abort(_) => {
                        game.delete(tc).await?;
                        let mut game_copy = game.clone();
                        game_copy.finished = true;
                        game_copy
                    }
                    _ => unreachable!(),
                };

                send_control_messages(
                    ws_server.clone(),
                    &result_game,
                    &bot,
                    &pool,
                    game_control.clone(),
                )
                .await?;

                Ok(result_game)
            }
            .scope_boxed()
        })
        .await?;

    Ok(updated_game)
}
