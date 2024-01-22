use crate::common::game_action::GameAction;
use crate::websockets::turn_handler::TurnHandler;
use anyhow::Result;
use db_lib::{models::game::Game, DbPool};
use hive_lib::{game_error::GameError, game_status::GameStatus};
use std::str::FromStr;
use uuid::Uuid;

use super::internal_server_message::InternalServerMessage;
use super::messages::WsMessage;
use super::{game_control_handler::GameControlHandler, join_handler::JoinHandler};

pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    pool: DbPool,
    user_id: Uuid,
    received_from: actix::Recipient<WsMessage>,
    username: String,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &str,
        game_action: GameAction,
        username: &str,
        user_id: Uuid,
        received_from: actix::Recipient<WsMessage>,
        pool: &DbPool,
    ) -> Result<Self> {
        let game = Game::find_by_nanoid(game_id, pool).await?;
        Ok(Self {
            pool: pool.clone(),
            game,
            username: username.to_owned(),
            game_action,
            received_from,
            user_id,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.game_action.clone() {
            GameAction::Move(turn) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                let th = TurnHandler::new(
                    turn,
                    self.game.clone(),
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .await?;
                th.handle().await?
            }
            GameAction::Control(control) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                let gch = GameControlHandler::new(
                    &control,
                    self.game.clone(),
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .await;
                gch.handle().await?
            }
            GameAction::Join => {
                let jh = JoinHandler::new(
                    self.game.clone(),
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    &self.pool,
                )
                .await;
                jh.handle().await?
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
