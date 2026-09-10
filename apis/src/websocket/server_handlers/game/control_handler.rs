use super::tournament_progression::{project_committed_game, GameCommandContext};
use crate::{
    common::GameReaction,
    websocket::{messages::HandlerOutput, WebsocketData, WsHub},
};
use anyhow::Result;
use db_lib::{
    game_command::{self, Command},
    get_conn,
    models::Game,
    DbPool,
};
use hive_lib::GameControl;
use shared_types::GameId;
use std::sync::Arc;
use uuid::Uuid;

pub struct GameControlHandler {
    control: GameControl,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
}

impl GameControlHandler {
    pub fn new(
        control: &GameControl,
        game: &Game,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            control: control.to_owned(),
            data,
            hub,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let pending_delete =
            if matches!(self.control, GameControl::Abort(_)) && self.game.tournament_id.is_none() {
                Some(self.hub.arm_pending_delete(
                    GameId(self.game.nanoid.clone()),
                    self.game.white_id,
                    self.game.black_id,
                ))
            } else {
                None
            };
        let outcome = game_command::execute(
            self.game.id,
            Command::Control {
                user_id: self.user_id,
                control: self.control,
            },
            &mut conn,
        )
        .await?;
        let context = GameCommandContext::websocket(
            GameReaction::Control(self.control),
            self.user_id,
            self.username.clone(),
        );
        let mut committed =
            project_committed_game(outcome, context, self.data.as_ref(), &mut conn).await;
        if committed.removed {
            if let Some(guard) = pending_delete {
                guard.disarm();
            }
        }
        if let Some(reason) = committed.rejected.take() {
            committed.output.request_error = Some(reason.into());
            return Ok(committed.output);
        }
        Ok(committed.output)
    }
}
