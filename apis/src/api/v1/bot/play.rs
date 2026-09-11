use crate::{
    api::v1::auth::Auth,
    common::GameReaction,
    websocket::{
        server_handlers::game::{project_committed_game, GameCommandContext},
        HandlerOutput,
        WsHub,
    },
};
use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse,
};
use anyhow::{anyhow, Result};
use db_lib::{
    db_error::DbError,
    game_command::{self, Command},
    get_conn,
    models::{Game, User},
    DbPool,
};
use hive_lib::{Color, GameControl, Piece, Position, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;
use std::{str::FromStr, sync::Arc};

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
    hub: Data<Arc<WsHub>>,
) -> HttpResponse {
    let mut output = HandlerOutput::empty();
    let result = play_move(req, bot.clone(), pool, hub.clone(), &mut output).await;
    hub.dispatch_handler_output(output).await;
    match result {
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

fn bot_db_error(error: DbError) -> anyhow::Error {
    match error {
        DbError::InternalError { .. }
        | DbError::InvalidPersistedTournament { .. }
        | DbError::SerializationConflict => anyhow!("Internal database error"),
        error => error.into(),
    }
}

async fn play_move(
    play: PlayRequest,
    bot: User,
    pool: Data<DbPool>,
    hub: Data<Arc<WsHub>>,
    output: &mut HandlerOutput,
) -> Result<(Game, Turn)> {
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&play.game_id, &mut conn)
        .await
        .map_err(bot_db_error)?;
    let state = State::new_from_str(&game.history, &game.game_type)?;
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
    let outcome = game_command::execute(
        game.id,
        Command::Move {
            user_id: bot.id,
            turn: played_turn.clone(),
            compensation: 0.0,
        },
        &mut conn,
    )
    .await
    .map_err(bot_db_error)?;
    let context = GameCommandContext::bot(
        GameReaction::Turn(played_turn.clone()),
        bot.id,
        bot.username.clone(),
    );
    let committed = project_committed_game(outcome, context, hub.data.as_ref(), &mut conn).await;
    if committed.removed {
        unreachable!("a bot move cannot remove its game")
    }
    output.append(committed.output);
    if let Some(rejected) = committed.rejected {
        return Err(bot_db_error(rejected));
    }
    Ok((committed.game, played_turn))
}

#[post("/api/v1/bot/games/control")]
pub async fn api_control(
    Json(req): Json<ControlRequest>,
    Auth(bot): Auth,
    pool: Data<DbPool>,
    hub: Data<Arc<WsHub>>,
) -> HttpResponse {
    let mut output = HandlerOutput::empty();
    let result = handle_control(req, bot.clone(), pool, hub.clone(), &mut output).await;
    hub.dispatch_handler_output(output).await;
    match result {
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

fn bot_color(game: &Game, bot_id: uuid::Uuid) -> Result<Color> {
    if game.white_id == bot_id {
        Ok(Color::White)
    } else if game.black_id == bot_id {
        Ok(Color::Black)
    } else {
        Err(anyhow!("Not your game"))
    }
}

fn control_from_request(game: &Game, bot_id: uuid::Uuid, requested: &str) -> Result<GameControl> {
    let color = bot_color(game, bot_id)?;
    match requested {
        "resign" => Ok(GameControl::Resign(color)),
        "abort" => Ok(GameControl::Abort(color)),
        _ => Err(anyhow!("Invalid control type: {requested}")),
    }
}

async fn handle_control(
    req: ControlRequest,
    bot: User,
    pool: Data<DbPool>,
    hub: Data<Arc<WsHub>>,
    output: &mut HandlerOutput,
) -> Result<Game> {
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&req.game_id, &mut conn)
        .await
        .map_err(bot_db_error)?;
    let game_control = control_from_request(&game, bot.id, &req.control)?;
    let pending_delete =
        if matches!(game_control, GameControl::Abort(_)) && game.tournament_id.is_none() {
            Some(hub.as_ref().arm_pending_delete(
                GameId(game.nanoid.clone()),
                game.white_id,
                game.black_id,
            ))
        } else {
            None
        };
    let outcome = game_command::execute(
        game.id,
        Command::Control {
            user_id: bot.id,
            control: game_control,
        },
        &mut conn,
    )
    .await
    .map_err(bot_db_error)?;
    let context = GameCommandContext::bot(
        GameReaction::Control(game_control),
        bot.id,
        bot.username.clone(),
    );
    let committed = project_committed_game(outcome, context, hub.data.as_ref(), &mut conn).await;
    if committed.removed {
        if let Some(guard) = pending_delete {
            guard.disarm();
        }
    }
    output.append(committed.output);
    if let Some(rejected) = committed.rejected {
        return Err(bot_db_error(rejected));
    }
    Ok(committed.game)
}
