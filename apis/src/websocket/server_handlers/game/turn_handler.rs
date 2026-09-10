use super::tournament_progression::{project_committed_game, GameCommandContext};
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
use hive_lib::Turn;
use shared_types::{GameId, TimeMode};
use std::sync::Arc;
use uuid::Uuid;

pub struct TurnHandler {
    turn: Turn,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
}

impl TurnHandler {
    pub fn new(
        turn: Turn,
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
            turn,
            data,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let outcome = game_command::execute(
            self.game.id,
            Command::Move {
                user_id: self.user_id,
                turn: self.turn.clone(),
                compensation: self.compensation(),
            },
            &mut conn,
        )
        .await?;
        let context = GameCommandContext::websocket(
            GameReaction::Turn(self.turn.clone()),
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
