use super::{
    messages::{MessageDestination, Outbound, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::{DisconnectReason, WsTelemetry},
    ws_hub::WsHub,
    WebsocketData,
};
use crate::{
    api::v1::auth::{decode::jwt_decode, jwt_secret::JwtSecret},
    common::{
        ClientRequest,
        ExternalServerError,
        GameAction,
        ServerMessage,
        ServerResult,
        SubscriptionError,
    },
};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use codee::{binary::MsgpackSerdeCodec, Decoder};
use db_lib::{get_conn, models::User, DbPool};
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::{ConversationKey, GameThread, SimpleUser};
use std::{
    cell::Cell,
    sync::{Arc, Mutex},
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
    identity: Arc<Mutex<SimpleUser>>,
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
        // Read after any `Auth` swap, so cleanup unbinds the user the socket ended as.
        let user = self
            .identity
            .lock()
            .expect("identity mutex poisoned")
            .clone();
        self.hub.on_disconnect(self.socket_id, user);
    }
}

/// The four services every frame handler needs. They are built once per connection and
/// never vary, so they travel as one rather than as four repeated parameters.
pub struct Deps {
    pub hub: Arc<WsHub>,
    pub data: Arc<WebsocketData>,
    pub pool: DbPool,
    pub jwt_secret: Arc<JwtSecret>,
}

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: AggregatedMessageStream,
    socket: SocketTx,
    deps: Deps,
    user: SimpleUser,
) {
    let Deps {
        hub,
        data,
        pool,
        jwt_secret,
    } = deps;
    Arc::clone(&hub).on_connect(
        socket.socket_id,
        socket.tx.clone(),
        socket.format,
        user.clone(),
    );
    let identity = Arc::new(Mutex::new(user));

    let guard = DisconnectGuard {
        hub: Arc::clone(&hub),
        telemetry: data.telemetry.clone(),
        socket_id: socket.socket_id,
        identity: Arc::clone(&identity),
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
                    handle_binary(&bytes, &hub, &socket, &data, &pool, &jwt_secret, &identity).await;
                }
                Some(Ok(AggregatedMessage::Close(_))) => break,
                None => {
                    guard.set_reason(DisconnectReason::StreamErr);
                    break;
                }
                Some(Ok(AggregatedMessage::Text(text))) => {
                    last_hb = Instant::now();
                    handle_text(&text, &hub, &socket, &data, &pool, &jwt_secret, &identity)
                        .await;
                }
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
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<SimpleUser>>,
) {
    data.telemetry.record_message_received(bytes.len());

    let Ok(request) = MsgpackSerdeCodec::decode(bytes) else {
        return;
    };
    handle_request(request, hub, socket, data, pool, jwt_secret, identity).await;
}

async fn handle_text(
    text: &str,
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    data: &Arc<WebsocketData>,
    pool: &DbPool,
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<SimpleUser>>,
) {
    data.telemetry.record_message_received(text.len());

    let Ok(request) = serde_json::from_str(text) else {
        return;
    };
    handle_request(request, hub, socket, data, pool, jwt_secret, identity).await;
}

async fn handle_request(
    request: ClientRequest,
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    data: &Arc<WebsocketData>,
    pool: &DbPool,
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<SimpleUser>>,
) {
    if let ClientRequest::Auth(token) = request {
        handle_auth(&token, hub, socket, pool, jwt_secret, identity).await;
        return;
    }

    let user = identity.lock().expect("identity mutex poisoned").clone();

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
                let message = ServerResult::Ok(Box::new(message.message));
                hub.dispatch_out(&destination, &mut Outbound::typed(&message))
                    .await;
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
            hub.dispatch_out(
                &MessageDestination::Direct(socket.clone()),
                &mut Outbound::typed(&message),
            )
            .await;
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

async fn send_direct(hub: &Arc<WsHub>, socket: &SocketTx, message: ServerMessage) {
    let message = ServerResult::Ok(Box::new(message));
    hub.dispatch_out(
        &MessageDestination::Direct(socket.clone()),
        &mut Outbound::typed(&message),
    )
    .await;
}

async fn handle_auth(
    token: &str,
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    pool: &DbPool,
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<SimpleUser>>,
) {
    let failed = |hub: &Arc<WsHub>, socket: &SocketTx| {
        let hub = hub.clone();
        let socket = socket.clone();
        async move {
            send_direct(
                &hub,
                &socket,
                ServerMessage::Error("Auth failed".to_string()),
            )
            .await;
        }
    };

    let Ok(email) = jwt_decode(token, &jwt_secret.decoding) else {
        failed(hub, socket).await;
        return;
    };
    let Ok(mut conn) = get_conn(pool).await else {
        failed(hub, socket).await;
        return;
    };
    let Ok(user) = User::find_for_login(&email, &mut conn).await else {
        failed(hub, socket).await;
        return;
    };
    // The same gate the HTTP bot API applies, so the socket cannot become a second way in
    // for a token the HTTP side would refuse.
    if !user.bot {
        failed(hub, socket).await;
        return;
    }

    let authenticated = SimpleUser {
        user_id: user.id,
        username: user.username.clone(),
        admin: user.admin,
        authed: true,
    };

    let previous = {
        let mut guard = identity.lock().expect("identity mutex poisoned");
        std::mem::replace(&mut *guard, authenticated.clone())
    };

    // Re-bind under the real user keeping socket_id and tx, so fanouts that route by socket
    // survive the swap.
    hub.on_disconnect(socket.socket_id, previous);
    Arc::clone(hub).on_connect(
        socket.socket_id,
        socket.tx.clone(),
        socket.format,
        authenticated,
    );

    // The cookie path gets a snapshot on connect; a bot authenticating later would otherwise
    // sit blind until something changed.
    hub.send_lobby_snapshot(&mut conn, user.id, socket, Some(&user))
        .await;
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

    #[tokio::test]
    async fn a_junk_token_leaves_the_socket_anonymous() {
        use crate::{
            api::v1::auth::jwt_secret::JwtSecret,
            websocket::{messages::SocketTx, ws_hub::WsHub, WebsocketData},
        };
        use shared_types::SimpleUser;
        use std::sync::{Arc, Mutex};

        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        let data = Arc::new(WebsocketData::default());
        let hub = WsHub::new(data.clone(), pool.clone());
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let socket = SocketTx {
            socket_id: Uuid::new_v4(),
            format: crate::websocket::messages::SocketFormat::Msgpack,
            tx,
        };
        let anonymous = SimpleUser {
            user_id: Uuid::nil(),
            username: "anonymous".to_string(),
            admin: false,
            authed: false,
        };
        let identity = Arc::new(Mutex::new(anonymous.clone()));
        let jwt_secret = Arc::new(JwtSecret::new("test-secret".to_string()));

        // Decoding fails before any database access, so the unreachable pool is never touched.
        super::handle_auth("not-a-jwt", &hub, &socket, &pool, &jwt_secret, &identity).await;

        let after = identity.lock().expect("identity mutex poisoned").clone();
        assert!(!after.authed);
        assert_eq!(after.user_id, anonymous.user_id);
    }
}
