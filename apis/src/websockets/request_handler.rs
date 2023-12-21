use super::{
    auth_error::AuthError, challenge_handler::ChallengeHandler,
    game_action_handler::GameActionHandler, user_status_handler::UserStatusHandler,
};
use crate::common::{
    client_message::ClientRequest, game_action::GameAction, server_result::InternalServerMessage,
};
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
            ClientRequest::Game {
                action: game_action,
                id: game_id,
            } => {
                match game_action {
                    GameAction::Move(_) | GameAction::Control(_) => self.ensure_auth()?,
                    _ => {}
                };
                let handler = GameActionHandler::new(
                    &game_id,
                    game_action,
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .await?;
                handler.handle().await?
            }
            ClientRequest::Challenge(_challenge_action) => {
                self.ensure_auth()?;
                let handler = ChallengeHandler::new().await?;
                handler.handle().await?
            }
            ClientRequest::Away => {
                let handler = UserStatusHandler::new().await?;
                handler.handle().await?
            }
        };
        Ok(messages)
    }
}
