use std::sync::Arc;

use super::{
    challenges::handler::ChallengeHandler,
    chat::handler::ChatHandler,
    game::handler::GameActionHandler,
    oauth::handler::OauthHandler,
    resync::ResyncHandler,
    schedules::ScheduleHandler,
    tournaments::handler::TournamentHandler,
    user_status::handler::UserStatusHandler,
};
use crate::{
    chat::access::{
        allows_anonymous_chat_read,
        authorize_chat_send,
        can_anonymous_read_chat,
        can_user_read_chat,
        ChatAccessError,
    },
    common::{ClientRequest, GameAction},
    websocket::{
        messages::{AuthError, GameSubscription, HandlerOutput, SocketTx},
        WebsocketData,
        WsHub,
    },
};
use anyhow::anyhow;
use db_lib::{get_conn, DbPool};
use shared_types::{ConversationKey, SimpleUser};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    ChatInternalError(anyhow::Error),
    AuthError(#[from] AuthError),
    Forbidden(String),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::ChatInternalError(e) => write!(f, "{e}"),
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
            RequestHandlerError::Forbidden(e) => write!(f, "{e}"),
        }
    }
}

impl RequestHandlerError {
    pub fn user_safe_reason(&self) -> String {
        match self {
            Self::InternalError(_) => "Unable to complete request".to_string(),
            Self::ChatInternalError(_) => "Unable to complete chat request".to_string(),
            Self::AuthError(error) => error.to_string(),
            Self::Forbidden(reason) => reason.clone(),
        }
    }
}
pub struct RequestHandler {
    command: ClientRequest,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
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
        hub: Arc<WsHub>,
        sender_addr: SocketTx,
        user: SimpleUser,
        pool: DbPool,
    ) -> Self {
        Self {
            received_from: sender_addr,
            command,
            data,
            hub,
            pool,
            user_id: user.user_id,
            username: user.username,
            authed: user.authed,
            admin: user.admin,
        }
    }

    fn ensure_auth(&self) -> Result<()> {
        if !self.authed || self.hub.is_user_revoked(self.user_id) {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    fn map_chat_access_error(error: ChatAccessError) -> RequestHandlerError {
        match error {
            ChatAccessError::BadRequest(message)
            | ChatAccessError::Forbidden(message)
            | ChatAccessError::NotFound(message) => {
                RequestHandlerError::Forbidden(message.to_string())
            }
            ChatAccessError::Internal { context, error } => {
                RequestHandlerError::ChatInternalError(anyhow!("{context}: {error}"))
            }
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let output: HandlerOutput = match self.command.clone() {
            ClientRequest::LinkDiscord => {
                self.ensure_auth()?;
                OauthHandler::new(self.user_id).handle().await?.into()
            }
            ClientRequest::Chat(mut message_container) => {
                self.ensure_auth()?;
                if self.user_id != message_container.message.user_id {
                    Err(AuthError::Unauthorized)?
                }
                message_container.message.user_id = self.user_id;
                message_container.message.username = self.username.clone();
                let channel_key = ConversationKey::from_destination(&message_container.destination);
                let mut conn = get_conn(&self.pool).await.map_err(|error| {
                    RequestHandlerError::ChatInternalError(anyhow!(
                        "Database connection failed while handling chat: {error}"
                    ))
                })?;
                let target = authorize_chat_send(&mut conn, self.user_id, self.admin, &channel_key)
                    .await
                    .map_err(Self::map_chat_access_error)?;
                ChatHandler::new(message_container, target)
                    .handle(&mut conn)
                    .await
                    .map_err(RequestHandlerError::ChatInternalError)?
                    .into()
            }
            ClientRequest::ChatSubscribe(channel_key) => {
                let anonymous = if self.authed {
                    self.ensure_auth()?;
                    false
                } else if allows_anonymous_chat_read(&channel_key) {
                    true
                } else {
                    Err(AuthError::Unauthorized)?
                };
                let mut conn = get_conn(&self.pool).await.map_err(|error| {
                    RequestHandlerError::ChatInternalError(anyhow!(
                        "Database connection failed while subscribing to chat: {error}"
                    ))
                })?;
                let (can_read, _) = if anonymous {
                    can_anonymous_read_chat(&mut conn, &channel_key).await
                } else {
                    can_user_read_chat(&mut conn, self.user_id, &channel_key).await
                }
                .map_err(Self::map_chat_access_error)?;
                if !can_read {
                    return Err(RequestHandlerError::Forbidden(
                        "You cannot read this chat".to_string(),
                    ));
                }
                HandlerOutput {
                    subscriptions: vec![GameSubscription::Chat(channel_key)],
                    ..HandlerOutput::empty()
                }
            }
            ClientRequest::ChatUnsubscribe(channel_key) => {
                self.hub
                    .unsubscribe_chat(self.user_id, self.received_from.socket_id, &channel_key);
                HandlerOutput::empty()
            }
            ClientRequest::Tournament(tournament_action) => {
                self.ensure_auth()?;
                TournamentHandler::new(
                    tournament_action,
                    &self.username,
                    self.user_id,
                    self.data.clone(),
                    self.hub.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Pong(nonce) => {
                self.data.pings.update(self.user_id, nonce);
                HandlerOutput::empty()
            }
            ClientRequest::Resync => {
                ResyncHandler::new(
                    self.hub.clone(),
                    self.pool.clone(),
                    self.received_from.clone(),
                    self.user_id,
                    self.authed,
                )
                .handle()
                .await?
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
                    self.hub.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Challenge(challenge_action) => {
                self.ensure_auth()?;
                ChallengeHandler::new(
                    challenge_action,
                    &self.username,
                    self.user_id,
                    self.admin,
                    &self.pool,
                )
                .await?
                .handle()
                .await?
                .into()
            }
            ClientRequest::NotificationSeen { game_id } => {
                self.ensure_auth()?;
                self.data
                    .pending_notifications
                    .mark_seen(self.user_id, &game_id);
                HandlerOutput::empty()
            }
            ClientRequest::Away => UserStatusHandler::new().await?.handle().await?.into(),
            ClientRequest::Schedule(action) => {
                match action {
                    crate::common::ScheduleAction::TournamentPublic(_) => {}
                    _ => self.ensure_auth()?,
                }
                ScheduleHandler::new(self.user_id, action, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
        };
        Ok(output)
    }
}
