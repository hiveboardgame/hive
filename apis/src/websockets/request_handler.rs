use super::game_timeout_handler::GameTimeoutHandler;
use super::{
    auth_error::AuthError, game_action_handler::GameActionHandler,
    user_status_handler::UserStatusHandler,
};
use crate::common::{
    client_message::ClientRequest, game_action::GameAction, server_result::InternalServerMessage,
};
use crate::websockets::api::challenges::handler::ChallengeHandler;
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct RequestHandler {
    command: ClientRequest,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    authed: bool,
}

impl RequestHandler {
    pub fn new(
        command: ClientRequest,
        user_id: Uuid,
        username: &str,
        authed: bool,
        pool: DbPool,
    ) -> Self {
        Self {
            command,
            pool,
            user_id,
            username: username.to_owned(),
            authed,
        }
    }

    fn ensure_auth(&self) -> Result<()> {
        if !self.authed {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.command.clone() {
            ClientRequest::GameTimeout(nanoid) => {
                GameTimeoutHandler::new(&nanoid, &self.username, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ClientRequest::Game {
                action: game_action,
                id: game_id,
            } => {
                match game_action {
                    GameAction::Move(_) | GameAction::Control(_) => self.ensure_auth()?,
                    _ => {}
                };
                GameActionHandler::new(
                    &game_id,
                    game_action,
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Challenge(challenge_action) => {
                self.ensure_auth()?;
                ChallengeHandler::new(challenge_action, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ClientRequest::Away => UserStatusHandler::new().await?.handle().await?,
        };
        Ok(messages)
    }
}
