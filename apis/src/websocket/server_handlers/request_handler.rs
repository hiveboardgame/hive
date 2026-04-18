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
    chat::access::{authorize_chat_send_and_resolve_channel_key, ChatSendAccessError},
    common::{ClientRequest, GameAction},
    websocket::{
        messages::{AuthError, InternalServerMessage, WsMessage},
        WebsocketData,
    },
};
use anyhow::anyhow;
use db_lib::{get_conn, DbPool};
use shared_types::SimpleUser;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    AuthError(#[from] AuthError),
    /// Operation not allowed (e.g. recipient has blocked the sender for DMs). Use 403, not 401.
    Forbidden(String),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
            RequestHandlerError::Forbidden(msg) => write!(f, "{msg}"),
        }
    }
}

impl RequestHandlerError {
    pub fn user_safe_reason(&self) -> String {
        match self {
            Self::InternalError(_) => "Unable to complete chat request".to_string(),
            Self::AuthError(err) => err.to_string(),
            Self::Forbidden(message) => message.clone(),
        }
    }
}

pub struct RequestHandler {
    command: ClientRequest,
    data: Arc<WebsocketData>,
    received_from: actix::Recipient<WsMessage>, // This is the socket the message was received over
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
        sender_addr: actix::Recipient<WsMessage>,
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

    fn db_connection_error(context: &str, err: impl std::fmt::Display) -> RequestHandlerError {
        RequestHandlerError::InternalError(anyhow!(
            "Database connection failed while {context}: {err}"
        ))
    }

    fn map_chat_send_access_error(err: ChatSendAccessError) -> RequestHandlerError {
        match err {
            ChatSendAccessError::BadRequest(message)
            | ChatSendAccessError::Forbidden(message)
            | ChatSendAccessError::NotFound(message) => {
                RequestHandlerError::Forbidden(message.to_string())
            }
            ChatSendAccessError::Internal { context, error } => {
                RequestHandlerError::InternalError(anyhow!("{context}: {error}"))
            }
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.command.clone() {
            ClientRequest::LinkDiscord => OauthHandler::new(self.user_id).handle().await?,
            ClientRequest::Chat(message_container) => {
                self.ensure_auth()?;
                if self.user_id != message_container.message.user_id {
                    Err(AuthError::Unauthorized)?
                }
                let channel_key = shared_types::ConversationKey::from_destination(
                    &message_container.destination,
                );
                let mut conn = get_conn(&self.pool)
                    .await
                    .map_err(|e| Self::db_connection_error("checking chat send permissions", e))?;
                let resolved_channel_key = authorize_chat_send_and_resolve_channel_key(
                    &mut conn,
                    self.user_id,
                    self.admin,
                    &channel_key,
                )
                .await
                .map_err(Self::map_chat_send_access_error)?;
                ChatHandler::new(
                    message_container,
                    resolved_channel_key,
                    self.data.clone(),
                    self.pool.clone(),
                )
                    .handle()
                    .await?
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
