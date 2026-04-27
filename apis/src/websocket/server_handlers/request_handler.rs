use std::sync::Arc;

use super::{
    challenges::handler::ChallengeHandler,
    chat::handler::ChatHandler,
    game::handler::GameActionHandler,
    oauth::handler::OauthHandler,
    schedules::ScheduleHandler,
    tournaments::handler::TournamentHandler,
    user_status::handler::UserStatusHandler,
};
use crate::{
    common::{ClientRequest, GameAction},
    websocket::{
        messages::{AuthError, InternalServerMessage, SocketTx},
        WebsocketData,
    },
};
use db_lib::DbPool;
use shared_types::{ChatDestination, SimpleUser};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    AuthError(#[from] AuthError),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
        }
    }
}
pub struct RequestHandler {
    command: ClientRequest,
    data: Arc<WebsocketData>,
    received_from: SocketTx,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    authed: bool,
    admin: bool,
}
type Result<T> = std::result::Result<T, RequestHandlerError>;
impl RequestHandler {
    pub fn new(
        command: ClientRequest,
        data: Arc<WebsocketData>,
        sender_addr: SocketTx,
        user: SimpleUser,
        pool: DbPool,
    ) -> Self {
        Self {
            received_from: sender_addr,
            command,
            data,
            pool,
            user_id: user.user_id,
            username: user.username,
            authed: user.authed,
            admin: user.admin,
        }
    }

    fn ensure_auth(&self) -> Result<()> {
        if !self.authed {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    fn ensure_admin(&self) -> Result<()> {
        if !self.admin {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.command.clone() {
            ClientRequest::LinkDiscord => OauthHandler::new(self.user_id).handle().await?,
            ClientRequest::Chat(message_container) => {
                self.ensure_auth()?;
                if self.user_id != message_container.message.user_id {
                    Err(AuthError::Unauthorized)?
                }
                if message_container.destination == ChatDestination::Global {
                    self.ensure_admin()?;
                }
                ChatHandler::new(message_container, self.data.clone()).handle()
            }
            ClientRequest::Tournament(tournament_action) => {
                TournamentHandler::new(
                    tournament_action,
                    &self.username,
                    self.user_id,
                    self.data.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Pong(nonce) => {
                self.data.pings.update(self.user_id, nonce);
                vec![]
            }
            ClientRequest::Game {
                action: game_action,
                game_id,
            } => {
                match game_action {
                    GameAction::Turn(_) | GameAction::Control(_) => self.ensure_auth()?,
                    _ => {}
                };
                GameActionHandler::new(
                    &game_id,
                    game_action,
                    self.received_from.clone(),
                    (&self.username, self.user_id),
                    self.data.clone(),
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
            ClientRequest::Schedule(action) => {
                match action {
                    crate::common::ScheduleAction::TournamentPublic(_) => {}
                    _ => self.ensure_auth()?,
                }
                ScheduleHandler::new(self.user_id, action, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
        };
        Ok(messages)
    }
}
