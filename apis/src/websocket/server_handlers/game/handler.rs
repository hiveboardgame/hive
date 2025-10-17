use super::start::StartHandler;
use super::{
    control_handler::GameControlHandler, timeout_handler::TimeoutHandler, turn_handler::TurnHandler,
};
use crate::common::GameAction;
use crate::websocket::{messages::InternalServerMessage, new_style::server::ClientData};
use anyhow::Result;
use db_lib::{get_conn, models::Game};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use hive_lib::{GameError, GameStatus};
use shared_types::GameId;
use std::str::FromStr;
pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    client: ClientData,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &GameId,
        game_action: GameAction,
        client: ClientData,
    ) -> Result<Self> {
        let mut connection = get_conn(client.pool()).await?;
        let game = connection
            .transaction::<_, anyhow::Error, _>(move |conn| {
                // find_by_game_id automatically times the game out if needed
                async move { Ok(Game::find_by_game_id(game_id, conn).await?) }.scope_boxed()
            })
            .await?;
        Ok(Self {
            game,
            game_action,
            client: client.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let (username, user_id, pool) = {
            let c = self.client.account().map(|u| (u.username.as_str(), u.id)).unwrap_or_default();
            (c.0, c.1, self.client.pool())
        };
        let messages = match self.game_action.clone() {
            GameAction::CheckTime => {
                TimeoutHandler::new(&self.game, username, user_id, pool)
                    .handle()
                    .await?
            }
            GameAction::Turn(turn) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                TurnHandler::new(turn, &self.game, username, user_id, pool)
                    .handle()
                    .await?
            }
            GameAction::Control(control) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                GameControlHandler::new(
                    &control,
                    &self.game,
                    username,
                    user_id,
                    pool,
                )
                .handle()
                .await?
            }
            GameAction::Join => {
                vec![]
            }
            GameAction::Start => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                StartHandler::new(&self.game, user_id, username.to_string(), pool)
                    .handle()
                    .await?
            }
        };
        Ok(messages)
    }

    fn ensure_not_finished(&self) -> Result<()> {
        let username = self.client.account().map(|u| u.username.clone()).unwrap_or_default();
        if let GameStatus::Finished(_) | GameStatus::Adjudicated =
            GameStatus::from_str(&self.game.game_status).unwrap()
        {
            Err(GameError::GameIsOver {
                username,
                game: self.game.nanoid.to_owned(),
            })?;
        }
        Ok(())
    }

    fn ensure_user_is_player(&self) -> Result<()> {
        let (username, id) = self.client.account().map(|u| (u.username.clone(),u.id)).unwrap_or_default();
        if !self.game.user_is_player(id) {
            Err(GameError::NotPlayer {
                username,
                game: self.game.nanoid.clone(),
            })?;
        }
        Ok(())
    }
}
