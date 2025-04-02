use super::start::StartHandler;
use super::{
    control_handler::GameControlHandler, join_handler::JoinHandler,
    timeout_handler::TimeoutHandler, turn_handler::TurnHandler,
};
use crate::common::GameAction;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::messages::WsMessage;
use crate::websocket::WebsocketData;
use anyhow::Result;
use db_lib::get_conn;
use db_lib::{models::Game, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::{GameError, GameStatus};
use shared_types::GameId;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;
pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    pool: DbPool,
    user_id: Uuid,
    received_from: actix::Recipient<WsMessage>,
    data: Arc<WebsocketData>,
    username: String,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &GameId,
        game_action: GameAction,
        user_details: (&str, Uuid),
        received_from: actix::Recipient<WsMessage>,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Result<Self> {
        let (username, user_id) = user_details;
        let mut connection = get_conn(pool).await?;

        let game = connection
            .transaction::<_, anyhow::Error, _>(move |conn| {
                // find_by_game_id automatically times the game out if needed
                async move { Ok(Game::find_by_game_id(game_id, conn).await?) }.scope_boxed()
            })
            .await?;

        Ok(Self {
            pool: pool.clone(),
            data,
            game,
            username: username.to_owned(),
            game_action,
            received_from,
            user_id,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.game_action.clone() {
            GameAction::CheckTime => {
                TimeoutHandler::new(&self.game, &self.username, self.user_id, &self.pool)
                    .handle()
                    .await?
            }
            GameAction::Turn(turn) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                TurnHandler::new(
                    turn,
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.data.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Control(control) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                GameControlHandler::new(
                    &control,
                    &self.game,
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Join => {
                JoinHandler::new(
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    self.data.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Start => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                StartHandler::new(
                    &self.game,
                    self.user_id,
                    self.username.clone(),
                    self.data.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
        };
        Ok(messages)
    }

    fn ensure_not_finished(&self) -> Result<()> {
        if let GameStatus::Finished(_) = GameStatus::from_str(&self.game.game_status).unwrap() {
            Err(GameError::GameIsOver {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
            })?;
        }
        Ok(())
    }

    fn ensure_user_is_player(&self) -> Result<()> {
        if !self.game.user_is_player(self.user_id) {
            Err(GameError::NotPlayer {
                username: self.username.to_owned(),
                game: self.game.nanoid.clone(),
            })?;
        }
        Ok(())
    }
}
