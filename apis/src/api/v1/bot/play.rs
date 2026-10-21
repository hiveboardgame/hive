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
    dev::Payload,
    post,
    web::{Data, Json},
    FromRequest,
    HttpRequest,
    HttpResponse,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{self, Command, ReceivedMove},
    get_conn,
    models::{Game, User},
    DbPool,
};
use hive_lib::{Color, GameControl, Piece, Position, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;
use std::{future::Future, pin::Pin, str::FromStr, sync::Arc};

#[derive(Serialize, Deserialize)]
struct PlayRequest {
    game_id: GameId,
    piece_pos: String,
}

// Actix extracts Auth alongside this, so stamping here keeps the bot's login lookup off its clock.
struct ArrivedPlay {
    request: PlayRequest,
    arrived_at: DateTime<Utc>,
}

impl FromRequest for ArrivedPlay {
    type Error = actix_web::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let request = Json::<PlayRequest>::from_request(req, payload);
        Box::pin(async move {
            let Json(request) = request.await?;
            Ok(Self {
                request,
                arrived_at: Utc::now(),
            })
        })
    }
}

#[derive(Serialize, Deserialize)]
struct ControlRequest {
    game_id: GameId,
    control: String,
}

#[post("/api/v1/bot/games/play")]
pub async fn api_play(
    arrived: ArrivedPlay,
    Auth(bot): Auth,
    pool: Data<DbPool>,
    hub: Data<Arc<WsHub>>,
) -> HttpResponse {
    let mut output = HandlerOutput::empty();
    let result = play_move(arrived, bot.clone(), pool, hub.clone(), &mut output).await;
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
    arrived: ArrivedPlay,
    bot: User,
    pool: Data<DbPool>,
    hub: Data<Arc<WsHub>>,
    output: &mut HandlerOutput,
) -> Result<(Game, Turn)> {
    let ArrivedPlay {
        request: play,
        arrived_at,
    } = arrived;
    // A deadline settled before the bot is known still allowed the full flag grace.
    let _received = ReceivedMove::record(&play.game_id.0, bot.id, arrived_at);
    let cloned_pool = pool.clone();
    let mut conn = get_conn(&cloned_pool).await?;
    let game = Game::find_by_game_id(&play.game_id, &mut conn)
        .await
        .map_err(bot_db_error)?;
    let _start = hub.data.realtime_gate.admit_start(&game)?;
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
    let outcome =
        game_command::execute(
            game.id,
            Command::Move {
                user_id: bot.id,
                turn: played_turn.clone(),
                compensation: 0.0,
                expected_turn: None,
                arrived_at: Some(arrived_at),
            },
            &mut conn,
        )
        .await
        .map_err(|error| {
            if matches!(
                &error,
                DbError::InvalidInput { .. } | DbError::InvalidAction { .. }
            ) {
                log::warn!(
                "Bot {} rejected move {:?} in game {}: {error:?}; observed turn {}, history {:?}",
                bot.username, played_turn, game.nanoid, game.turn, game.history,
            );
            }
            bot_db_error(error)
        })?;
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

#[cfg(test)]
mod tests {
    use super::ArrivedPlay;
    use actix_web::{dev::Payload, test, web, App, FromRequest, HttpRequest, HttpResponse};
    use chrono::Utc;
    use serde_json::json;
    use std::{future::Future, pin::Pin, time::Duration};

    const LOOKUP: Duration = Duration::from_millis(300);

    struct SlowLookup;

    impl FromRequest for SlowLookup {
        type Error = actix_web::Error;

        type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

        fn from_request(_: &HttpRequest, _: &mut Payload) -> Self::Future {
            Box::pin(async {
                tokio::time::sleep(LOOKUP).await;
                Ok(Self)
            })
        }
    }

    #[actix_web::test]
    async fn a_move_is_stamped_without_waiting_for_the_bot_lookup() {
        let app = test::init_service(App::new().route(
            "/",
            web::post().to(|_: SlowLookup, arrived: ArrivedPlay| async move {
                let waited = Utc::now() - arrived.arrived_at;
                HttpResponse::Ok().body(waited.num_milliseconds().to_string())
            }),
        ))
        .await;
        let request = test::TestRequest::post()
            .uri("/")
            .set_json(json!({"game_id": "game", "piece_pos": "wA1"}))
            .to_request();

        let body = test::call_and_read_body(&app, request).await;

        let waited: u128 = std::str::from_utf8(&body)
            .expect("utf-8")
            .parse()
            .expect("milliseconds");
        assert!(
            waited >= LOOKUP.as_millis() - 50,
            "stamped {waited} ms before the handler ran"
        );
    }
}
