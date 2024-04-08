use super::chat::handler::ChatHandler;
use super::game::handler::GameActionHandler;
use crate::common::{client_message::ClientRequest, game_action::GameAction};
use crate::websockets::api::challenges::handler::ChallengeHandler;
use crate::websockets::api::ping::handler::PingHandler;
use crate::websockets::api::user_status::handler::UserStatusHandler;
use crate::websockets::auth_error::AuthError;
use crate::websockets::internal_server_message::InternalServerMessage;
use crate::websockets::messages::WsMessage;
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct RequestHandler {
    command: ClientRequest,
    received_from: actix::Recipient<WsMessage>, // This is the socket the message was received over
    pool: DbPool,
    user_id: Uuid,
    username: String,
    authed: bool,
}

impl RequestHandler {
    pub fn new(
        command: ClientRequest,
        sender_addr: actix::Recipient<WsMessage>,
        user_id: Uuid,
        username: &str,
        authed: bool,
        pool: DbPool,
    ) -> Self {
        Self {
            received_from: sender_addr,
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
            ClientRequest::Chat(message) => ChatHandler::new(message).handle(),
            ClientRequest::Ping(sent) => PingHandler::new(self.user_id, sent).handle(),
            ClientRequest::Game {
                action: game_action,
                id: game_id,
            } => {
                match game_action {
                    GameAction::Turn(_) | GameAction::Control(_) => self.ensure_auth()?,
                    _ => {}
                };
                GameActionHandler::new(
                    &game_id,
                    game_action,
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Challenge(challenge_action) => {
                self.ensure_auth()?;
                ChallengeHandler::new(challenge_action, &self.username, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ClientRequest::Away => UserStatusHandler::new().await?.handle().await?,
        };
        Ok(messages)
    }
}
