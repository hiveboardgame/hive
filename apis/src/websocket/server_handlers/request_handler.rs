use std::sync::Arc;

use super::{
    challenges::handler::ChallengeHandler,
    chat::{
        handler::{ChatHandler, ChatHandlerError},
        limits::ChatLimitError,
    },
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
        authorize_chat_read,
        authorize_chat_send,
        ChatAccessError,
    },
    common::{ClientRequest, GameAction, ServerMessage},
    websocket::{
        messages::{AuthError, HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
        WsHub,
    },
};
use db_lib::{DbConn, DbPool};
use shared_types::{normalize_chat_message, ConversationKey, SimpleUser};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    ChatClientIdConflict,
    AuthError(#[from] AuthError),
    Forbidden,
    RateLimited(ChatLimitError),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::ChatClientIdConflict => {
                write!(f, "Chat client ID conflicts with an existing message")
            }
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
            RequestHandlerError::Forbidden => write!(f, "Chat access denied"),
            RequestHandlerError::RateLimited(error) => write!(f, "{}", error.reason()),
        }
    }
}

impl RequestHandlerError {
    pub fn user_safe_reason(&self) -> String {
        match self {
            Self::InternalError(_) => "Unable to complete request".to_string(),
            Self::ChatClientIdConflict => {
                "This message retry conflicts with the original delivery. Send it as a new message."
                    .to_string()
            }
            Self::AuthError(error) => error.to_string(),
            Self::Forbidden => "Chat access denied".to_string(),
            Self::RateLimited(error) => error.reason().to_string(),
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

fn normalize_chat_send_request(
    mut request: crate::common::ChatSendRequest,
) -> crate::common::ChatSendRequest {
    request.body = normalize_chat_message(&request.body);
    if !matches!(request.key, ConversationKey::Game { .. }) {
        request.turn = None;
    }
    request
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
            ChatAccessError::Denied => RequestHandlerError::Forbidden,
            ChatAccessError::Internal { context, error } => RequestHandlerError::InternalError(
                anyhow::Error::new(error).context(format!("chat {context}")),
            ),
        }
    }

    async fn chat_connection(&self, context: &'static str) -> Result<DbConn<'_>> {
        db_lib::get_conn(&self.pool).await.map_err(|error| {
            RequestHandlerError::InternalError(
                anyhow::Error::new(error)
                    .context(format!("chat database connection failed while {context}")),
            )
        })
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let output: HandlerOutput = match self.command.clone() {
            ClientRequest::LinkDiscord => {
                self.ensure_auth()?;
                OauthHandler::new(self.user_id).handle().await?.into()
            }
            ClientRequest::Chat(request) => {
                self.ensure_auth()?;
                self.hub
                    .check_chat_send(self.user_id, self.received_from.socket_id)
                    .map_err(RequestHandlerError::RateLimited)?;
                let request = normalize_chat_send_request(request);
                let channel_key = request.key.clone();
                let mut conn = self.chat_connection("handling chat").await?;
                let target = authorize_chat_send(&mut conn, self.user_id, self.admin, &channel_key)
                    .await
                    .map_err(Self::map_chat_access_error)?;
                ChatHandler::new(request, (&self.username, self.user_id), target)
                    .handle(&mut conn, &self.data.telemetry)
                    .await
                    .map_err(|error| match error {
                        ChatHandlerError::ClientIdConflict => {
                            RequestHandlerError::ChatClientIdConflict
                        }
                        ChatHandlerError::Internal(error) => RequestHandlerError::InternalError(
                            error.context("handling chat message"),
                        ),
                    })?
                    .into()
            }
            ClientRequest::ChatSubscribe(subscription) => {
                self.hub
                    .check_chat_subscription_request(self.received_from.socket_id)
                    .map_err(RequestHandlerError::RateLimited)?;
                let channel_key = &subscription.key;
                let reader_id = if self.authed {
                    self.ensure_auth()?;
                    Some(self.user_id)
                } else if allows_anonymous_chat_read(channel_key) {
                    None
                } else {
                    Err(AuthError::Unauthorized)?
                };
                let mut conn = self.chat_connection("subscribing to chat").await?;
                authorize_chat_read(&mut conn, reader_id, channel_key)
                    .await
                    .map_err(Self::map_chat_access_error)?;
                self.hub.subscribe_chat(
                    self.user_id,
                    self.received_from.socket_id,
                    &subscription.key,
                );
                vec![InternalServerMessage {
                    destination: MessageDestination::Direct(self.received_from.clone()),
                    message: ServerMessage::ChatSubscribed(subscription),
                }]
                .into()
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
                    self.hub.clone(),
                    &self.pool,
                )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{ChatSendRequest, ClientRequest, SubscriptionAttempt},
        websocket::{messages::SocketTx, WebsocketData},
    };
    use shared_types::ConversationKey;
    use tokio::sync::mpsc;

    async fn test_handler(command: ClientRequest, user: SimpleUser) -> RequestHandler {
        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        let data = Arc::new(WebsocketData::default());
        let hub = WsHub::new(data.clone(), pool.clone());
        let (tx, _rx) = mpsc::channel(8);
        RequestHandler::new(
            command,
            data,
            hub,
            SocketTx {
                socket_id: Uuid::new_v4(),
                tx,
            },
            user,
            pool,
        )
    }

    #[tokio::test]
    async fn anonymous_chat_send_is_denied_before_database_access() {
        let user_id = Uuid::nil();
        let command = ClientRequest::Chat(ChatSendRequest {
            key: ConversationKey::Global,
            client_id: Uuid::new_v4(),
            body: "hello".to_string(),
            turn: None,
        });
        let handler = test_handler(
            command,
            SimpleUser {
                user_id,
                username: "anonymous".to_string(),
                authed: false,
                admin: false,
            },
        )
        .await;

        assert!(matches!(
            handler.handle().await,
            Err(RequestHandlerError::AuthError(AuthError::Unauthorized))
        ));
    }

    #[tokio::test]
    async fn subscription_attempts_are_limited_before_database_authorization() {
        let user_id = Uuid::nil();
        let handler = test_handler(
            ClientRequest::ChatSubscribe(SubscriptionAttempt {
                key: ConversationKey::direct(Uuid::new_v4()),
                session_epoch: 1,
                request_id: 1,
            }),
            SimpleUser {
                user_id,
                username: "anonymous".to_string(),
                authed: false,
                admin: false,
            },
        )
        .await;

        for _ in 0..20 {
            assert!(matches!(
                handler.handle().await,
                Err(RequestHandlerError::AuthError(AuthError::Unauthorized))
            ));
        }
        assert!(matches!(
            handler.handle().await,
            Err(RequestHandlerError::RateLimited(error))
                if error.reason() == "Too many chat subscription attempts"
                    && error.retry_after() > std::time::Duration::ZERO
        ));
    }

    #[test]
    fn non_game_turn_is_cleared_during_server_normalization() {
        let request = normalize_chat_send_request(ChatSendRequest {
            key: ConversationKey::Global,
            client_id: Uuid::new_v4(),
            body: "valid\0body".to_string(),
            turn: Some(42),
        });

        assert_eq!(request.body, "validbody");
        assert_eq!(request.turn, None);
    }
}
