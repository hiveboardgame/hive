use super::projection::{project_committed_game, GameCommandContext};
use crate::{
    common::GameReaction,
    websocket::{messages::HandlerOutput, WebsocketData},
};
use anyhow::Result;
use db_lib::{
    game_command::{self, Command},
    get_conn,
    models::Game,
    DbPool,
};
use hive_lib::{GameError, Piece, Position, State, Turn};
use shared_types::{GameId, TimeMode};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

/// Bots send notation and the server resolves it against the replayed board, exactly as
/// `POST /api/v1/bot/games/play` does. The browser sends a resolved `Turn` because it runs
/// the engine itself; nothing else can.
pub enum TurnInput {
    Structured(Turn),
    Notation(String),
}

pub struct TurnHandler {
    input: TurnInput,
    expected_turn: Option<usize>,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
}

impl TurnHandler {
    pub fn new(
        input: TurnInput,
        expected_turn: Option<usize>,
        game: &Game,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            input,
            expected_turn,
            data,
        }
    }

    fn resolve(&self) -> Result<Turn> {
        let notation = match &self.input {
            TurnInput::Structured(turn) => return Ok(turn.clone()),
            TurnInput::Notation(notation) => notation,
        };
        let state = State::new_from_str(&self.game.history, &self.game.game_type)?;
        if state.turn == 0 {
            return Ok(Turn::Move(
                Piece::from_str(notation)?,
                Position::initial_spawn_position(),
            ));
        }
        let (piece, position) =
            notation
                .split_once(' ')
                .ok_or_else(|| GameError::ParsingError {
                    found: notation.clone(),
                    typ: String::from("move, expected 'piece position'"),
                })?;
        Ok(Turn::Move(
            Piece::from_str(piece)?,
            Position::from_string(position, &state.board)?,
        ))
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let turn = self.resolve()?;
        let mut conn = get_conn(&self.pool).await?;
        let outcome = game_command::execute(
            self.game.id,
            Command::Move {
                user_id: self.user_id,
                turn: turn.clone(),
                compensation: self.compensation(),
                expected_turn: self.expected_turn,
            },
            &mut conn,
        )
        .await?;
        let context = GameCommandContext::websocket(
            GameReaction::Turn(turn),
            self.user_id,
            self.username.clone(),
        );
        let mut committed =
            project_committed_game(outcome, context, self.data.as_ref(), &mut conn).await;
        if committed.removed {
            unreachable!("a move cannot remove its game")
        }
        if let Some(reason) = committed.rejected.take() {
            committed.output.request_error = Some(reason.into());
            return Ok(committed.output);
        }
        Ok(committed.output)
    }

    fn compensation(&self) -> f64 {
        if self.game.time_mode == TimeMode::RealTime.to_string() {
            let ping = self.data.pings.value(self.user_id);
            let base = self.game.time_base.unwrap_or(0) as usize;
            let inc = self.game.time_increment.unwrap_or(0) as usize;
            self.data
                .lags
                .track_lag(
                    self.user_id,
                    GameId(self.game.nanoid.clone()),
                    ping,
                    base,
                    inc,
                )
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }
}
