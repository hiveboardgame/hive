use super::{
    control_handler::GameControlHandler, join_handler::JoinHandler,
    timeout_handler::TimeoutHandler, turn_handler::TurnHandler,
};
use crate::websockets::internal_server_message::InternalServerMessage;
use crate::websockets::messages::WsMessage;
use crate::{common::game_action::GameAction, websockets::chat::Chats};
use anyhow::Result;
use db_lib::{models::game::Game, DbPool};
use hive_lib::{game_error::GameError, game_status::GameStatus};
use std::str::FromStr;
use uuid::Uuid;

pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    pool: DbPool,
    user_id: Uuid,
    received_from: actix::Recipient<WsMessage>,
    chat_storage: actix_web::web::Data<Chats>,
    username: String,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &str,
        game_action: GameAction,
        username: &str,
        user_id: Uuid,
        received_from: actix::Recipient<WsMessage>,
        chat_storage: actix_web::web::Data<Chats>,
        pool: &DbPool,
    ) -> Result<Self> {
        let game = Game::find_by_nanoid(game_id, pool).await?;
        Ok(Self {
            pool: pool.clone(),
            game,
            username: username.to_owned(),
            game_action,
            received_from,
            chat_storage,
            user_id,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.game_action.clone() {
            GameAction::CheckTime => {
                TimeoutHandler::new(self.game.clone(), &self.username, self.user_id, &self.pool)
                    .handle()
                    .await?
            }
            GameAction::Turn(turn) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                TurnHandler::new(
                    turn,
                    self.game.clone(),
                    &self.username,
                    self.user_id,
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
                    self.game.clone(),
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Join => {
                JoinHandler::new(
                    self.game.clone(),
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    self.chat_storage.clone(),
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
