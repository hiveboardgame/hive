use super::{
    messages::{GameSubscription, MessageDestination, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::{DisconnectReason, WsTelemetry},
    ws_hub::WsHub,
    WebsocketData,
};
use crate::common::{ClientRequest, ExternalServerError, GameAction, ServerResult};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::DbPool;
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::SimpleUser;
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
    user_uid: Uuid,
    username: String,
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
        self.hub.on_disconnect(
            self.socket_id,
            self.user_uid,
            std::mem::take(&mut self.username),
        );
    }
}

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: AggregatedMessageStream,
    socket: SocketTx,
    hub: Arc<WsHub>,
    data: Arc<WebsocketData>,
    pool: DbPool,
    user_uid: Uuid,
    username: String,
    admin: bool,
    authed: bool,
) {
    Arc::clone(&hub).on_connect(
        socket.socket_id,
        user_uid,
        username.clone(),
        socket.tx.clone(),
    );

    let guard = DisconnectGuard {
        hub: Arc::clone(&hub),
        telemetry: data.telemetry.clone(),
        socket_id: socket.socket_id,
        user_uid,
        username: username.clone(),
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
                    handle_binary(
                        &bytes,
                        &hub,
                        &socket,
                        &data,
                        &pool,
                        user_uid,
                        &username,
                        admin,
                        authed,
                    )
                    .await;
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
    user_id: Uuid,
    username: &str,
    admin: bool,
    authed: bool,
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
        hub.unsubscribe_game(user_id, socket.socket_id, game_id);
        return;
    }

    let user = SimpleUser {
        user_id,
        username: username.to_owned(),
        authed,
        admin,
    };
    let handler = RequestHandler::new(
        request.clone(),
        data.clone(),
        hub.clone(),
        socket.clone(),
        user,
        pool.clone(),
    );

    match handler.handle().await {
        Ok(output) => {
            let from = Some((user_id, socket.socket_id));
            for subscription in output.subscriptions {
                match subscription {
                    GameSubscription::Fanout(game_id) => {
                        hub.subscribe_game_fanout(user_id, socket.socket_id, &game_id);
                    }
                    GameSubscription::Heartbeat(game_id) => {
                        hub.subscribe_game_heartbeat(user_id, socket.socket_id, &game_id);
                    }
                }
            }
            for message in output.messages {
                let serialized = ServerResult::Ok(Box::new(message.message));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                    hub.dispatch(&message.destination, Bytes::from(serialized), from)
                        .await;
                }
            }
            // Reactions: one serialize, one Bytes allocation, refcount-cloned
            // across the three fanouts (both players + spectators). Dispatch
            // after `messages` so urgent state updates land first.
            for reaction in output.reactions {
                hub.dispatch_reaction(reaction, from).await;
            }
            // Finalize after dispatch so the opponent received the final
            // move/control via still-populated membership.
            for finalize in output.finalize_games {
                hub.finalize_game(&finalize.game_id, finalize.white_id, finalize.black_id);
            }
        }
        Err(err) => {
            let status_code = match err {
                RequestHandlerError::AuthError(_) => http::StatusCode::UNAUTHORIZED,
                _ => http::StatusCode::NOT_IMPLEMENTED,
            };
            printdoc! {r#"
                -----------------ERROR-----------------
                  Request: {:?}
                  Error:   {:?}
                  User:    {} {}
                ------------------END------------------
                "#,
                request, err, username, user_id
            };
            let message = ServerResult::Err(ExternalServerError {
                user_id,
                field: "foo".to_string(),
                reason: format!("{err}"),
                status_code,
            });
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                hub.dispatch(
                    &MessageDestination::User(user_id),
                    Bytes::from(serialized),
                    Some((user_id, socket.socket_id)),
                )
                .await;
            }
        }
    }
}
