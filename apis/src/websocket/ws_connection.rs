use super::{
    messages::{MessageDestination, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::{DisconnectReason, WsTelemetry},
    ws_hub::WsHub,
    WebsocketData,
};
use crate::common::{
    ClientRequest,
    ExternalServerError,
    GameAction,
    ServerResult,
    SubscriptionError,
};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::DbPool;
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::{ConversationKey, GameThread, SimpleUser};
use std::{
    cell::Cell,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// RAII cleanup so the WS subsystem stays consistent even if the reader loop
/// unwinds — without this, a panic anywhere inside `handle_binary` (a poisoned
/// lock, a handler `.unwrap()`) skips `on_disconnect` and leaks the user in
/// `sessions`/membership, leaving `active_sockets`/`active_users` overcounted
/// for the lifetime of the process.
struct DisconnectGuard {
    hub: Arc<WsHub>,
    telemetry: Arc<WsTelemetry>,
    socket_id: Uuid,
    user: SimpleUser,
    reason: Cell<DisconnectReason>,
}

impl DisconnectGuard {
    fn set_reason(&self, reason: DisconnectReason) {
        self.reason.set(reason);
    }
}

impl Drop for DisconnectGuard {
    fn drop(&mut self) {
        self.telemetry.record_disconnect(self.reason.get());
        self.hub.on_disconnect(self.socket_id, self.user.clone());
    }
}

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: AggregatedMessageStream,
    socket: SocketTx,
    hub: Arc<WsHub>,
    data: Arc<WebsocketData>,
    pool: DbPool,
    user: SimpleUser,
) {
    Arc::clone(&hub).on_connect(socket.socket_id, socket.tx.clone(), user.clone());

    let guard = DisconnectGuard {
        hub: Arc::clone(&hub),
        telemetry: data.telemetry.clone(),
        socket_id: socket.socket_id,
        user: user.clone(),
        reason: Cell::new(DisconnectReason::Close),
    };

    let mut last_hb = Instant::now();
    let mut hb_interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    loop {
        tokio::select! {
            _ = hb_interval.tick() => {
                if last_hb.elapsed() > CLIENT_TIMEOUT {
                    guard.set_reason(DisconnectReason::Timeout);
                    break;
                }
                let ping = tokio::time::timeout(HEARTBEAT_INTERVAL, session.ping(b"hi")).await;
                if matches!(ping, Err(_) | Ok(Err(_))) {
                    guard.set_reason(DisconnectReason::PingFail);
                    break;
                }
            }
            item = msg_stream.next() => match item {
                Some(Ok(AggregatedMessage::Ping(bytes))) => {
                    last_hb = Instant::now();
                    if session.pong(&bytes).await.is_err() {
                        guard.set_reason(DisconnectReason::PingFail);
                        break;
                    }
                }
                Some(Ok(AggregatedMessage::Pong(_))) => {
                    last_hb = Instant::now();
                }
                Some(Ok(AggregatedMessage::Binary(bytes))) => {
                    last_hb = Instant::now();
                    handle_binary(&bytes, &hub, &socket, &data, &pool, &user).await;
                }
                Some(Ok(AggregatedMessage::Close(_))) => break,
                None => {
                    guard.set_reason(DisconnectReason::StreamErr);
                    break;
                }
                Some(Ok(AggregatedMessage::Text(_))) => {}
                Some(Err(_)) => {
                    guard.set_reason(DisconnectReason::StreamErr);
                    break;
                }
            }
        }
    }

    drop(guard);
    let _ = session.close(None).await;
}

async fn handle_binary(
    bytes: &[u8],
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    data: &Arc<WebsocketData>,
    pool: &DbPool,
    user: &SimpleUser,
) {
    data.telemetry.record_message_received(bytes.len());

    let request: Result<ClientRequest, _> = MsgpackSerdeCodec::decode(bytes);
    let Ok(request) = request else {
        return;
    };

    // Unwatch needs hub access and no DB — handle it here before RequestHandler.
    if let ClientRequest::Game {
        ref game_id,
        action: GameAction::Unwatch,
    } = request
    {
        hub.unsubscribe_game(user.user_id, socket.socket_id, game_id);
        return;
    }

    let handler = RequestHandler::new(
        request.clone(),
        data.clone(),
        hub.clone(),
        socket.clone(),
        user.clone(),
        pool.clone(),
    );

    match handler.handle().await {
        Ok(output) => {
            for message in output.messages {
                let destination = message.destination;
                let serialized = ServerResult::Ok(Box::new(message.message));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                    hub.dispatch(&destination, Bytes::from(serialized)).await;
                }
            }
            // Reactions: one serialize, one Bytes allocation, refcount-cloned
            // across the three fanouts (both players + spectators). Dispatch
            // after `messages` so urgent state updates land first.
            for reaction in output.reactions {
                hub.dispatch_reaction(reaction).await;
            }
            // Finalize after dispatch so the opponent received the final
            // move/control via still-populated membership.
            for finalize in output.finalize_games {
                hub.finalize_game(&finalize.game_id, finalize.white_id, finalize.black_id);
            }
        }
        Err(err) => {
            if matches!(err, RequestHandlerError::RateLimited(_)) {
                hub.data.telemetry.record_chat_rate_limit_rejection();
            }
            if should_log_request_error(&err) {
                let request_summary = request_log_summary(&request);
                printdoc! {r#"
                    -----------------ERROR-----------------
                      Request: {}
                      Error:   {:?}
                      User:    {} {}
                    ------------------END------------------
                    "#,
                    request_summary, err, user.username, user.user_id
                };
            }
            let message = ServerResult::Err(external_server_error(&request, &err));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                hub.dispatch(
                    &MessageDestination::Direct(socket.clone()),
                    Bytes::from(serialized),
                )
                .await;
            }
        }
    }
}

fn external_server_error(
    request: &ClientRequest,
    error: &RequestHandlerError,
) -> ExternalServerError {
    if matches!(error, RequestHandlerError::AuthError(_))
        && !matches!(request, ClientRequest::ChatSubscribe(_))
    {
        return ExternalServerError::Unauthorized {
            reason: error.user_safe_reason(),
        };
    }

    match request {
        ClientRequest::Chat(request) => {
            let error = match error {
                RequestHandlerError::ChatClientIdConflict => {
                    crate::common::ChatSendError::ClientIdConflict
                }
                RequestHandlerError::RateLimited(_) => crate::common::ChatSendError::RateLimited,
                RequestHandlerError::Forbidden => match &request.key {
                    ConversationKey::Direct(_) => crate::common::ChatSendError::DirectRestricted,
                    ConversationKey::Global => crate::common::ChatSendError::AdminOnly,
                    ConversationKey::Tournament(_) => {
                        crate::common::ChatSendError::TournamentRestricted
                    }
                    ConversationKey::Game {
                        thread: GameThread::Players,
                        ..
                    } => crate::common::ChatSendError::PlayersRestricted,
                    ConversationKey::Game {
                        thread: GameThread::Spectators,
                        ..
                    } => crate::common::ChatSendError::SpectatorsRestricted,
                },
                RequestHandlerError::InternalError(_) => crate::common::ChatSendError::Unavailable,
                RequestHandlerError::AuthError(_) => crate::common::ChatSendError::Unavailable,
            };
            ExternalServerError::ChatSend {
                key: request.key.clone(),
                client_id: request.client_id,
                error,
            }
        }
        ClientRequest::ChatSubscribe(subscription) => {
            let subscription_error = match error {
                RequestHandlerError::RateLimited(error) => SubscriptionError::RateLimited {
                    retry_after: error.retry_after(),
                },
                RequestHandlerError::AuthError(_) | RequestHandlerError::Forbidden => {
                    SubscriptionError::AccessDenied
                }
                RequestHandlerError::InternalError(_)
                | RequestHandlerError::ChatClientIdConflict => SubscriptionError::Unavailable,
            };
            ExternalServerError::ChatSubscribe {
                attempt: subscription.clone(),
                error: subscription_error,
            }
        }
        _ => ExternalServerError::Request {
            reason: error.user_safe_reason(),
        },
    }
}

fn request_log_summary(request: &ClientRequest) -> String {
    match request {
        ClientRequest::Chat(request) => format!(
            "Chat(key={:?}, client_id={}, body_chars={})",
            request.key,
            request.client_id,
            request.body.chars().count()
        ),
        other => format!("{other:?}"),
    }
}

fn should_log_request_error(err: &RequestHandlerError) -> bool {
    !matches!(
        err,
        RequestHandlerError::AuthError(_)
            | RequestHandlerError::Forbidden
            | RequestHandlerError::RateLimited(_)
            | RequestHandlerError::ChatClientIdConflict
    )
}

#[cfg(test)]
mod tests {
    use super::external_server_error;
    use crate::{
        common::{
            ChatSendError,
            ChatSendRequest,
            ClientRequest,
            ExternalServerError,
            SubscriptionAttempt,
            SubscriptionError,
        },
        websocket::{
            messages::AuthError,
            server_handlers::{chat::limits::ChatLimitError, request_handler::RequestHandlerError},
        },
    };
    use shared_types::{ConversationKey, GameId};
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn subscription_rate_error_carries_typed_key_and_duration() {
        let key = ConversationKey::game_spectators(&GameId("limited-game".to_string()));
        let attempt = SubscriptionAttempt {
            key: key.clone(),
            session_epoch: 7,
            request_id: 3,
        };
        let request = ClientRequest::ChatSubscribe(attempt.clone());
        let error = RequestHandlerError::RateLimited(ChatLimitError::SubscriptionAttempts {
            retry_after: Duration::from_millis(250),
        });

        assert_eq!(
            external_server_error(&request, &error),
            ExternalServerError::ChatSubscribe {
                attempt,
                error: SubscriptionError::RateLimited {
                    retry_after: Duration::from_millis(250),
                },
            }
        );
    }

    #[test]
    fn subscription_auth_error_preserves_request_correlation() {
        let key = ConversationKey::game_spectators(&GameId("private-game".to_string()));
        let attempt = SubscriptionAttempt {
            key: key.clone(),
            session_epoch: 11,
            request_id: 27,
        };
        let request = ClientRequest::ChatSubscribe(attempt.clone());

        assert_eq!(
            external_server_error(
                &request,
                &RequestHandlerError::AuthError(AuthError::Unauthorized),
            ),
            ExternalServerError::ChatSubscribe {
                attempt,
                error: SubscriptionError::AccessDenied,
            },
        );
    }

    #[test]
    fn chat_send_error_carries_typed_policy() {
        let key = ConversationKey::game_spectators(&GameId("limited-game".to_string()));
        let client_id = Uuid::new_v4();
        let request = ClientRequest::Chat(ChatSendRequest {
            key: key.clone(),
            client_id,
            body: "hello".to_string(),
            turn: None,
        });
        let error = RequestHandlerError::Forbidden;

        assert!(matches!(
            external_server_error(&request, &error),
            ExternalServerError::ChatSend {
                key: candidate,
                client_id: candidate_client_id,
                error: ChatSendError::SpectatorsRestricted,
            } if candidate == key && candidate_client_id == client_id
        ));
    }

    #[test]
    fn client_id_conflict_remains_typed_on_the_wire() {
        let key = ConversationKey::direct(Uuid::new_v4());
        let client_id = Uuid::new_v4();
        let request = ClientRequest::Chat(ChatSendRequest {
            key: key.clone(),
            client_id,
            body: "hello".to_string(),
            turn: None,
        });

        assert_eq!(
            external_server_error(&request, &RequestHandlerError::ChatClientIdConflict),
            ExternalServerError::ChatSend {
                key,
                client_id,
                error: ChatSendError::ClientIdConflict,
            },
        );
    }
}
