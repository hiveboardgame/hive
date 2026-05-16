use super::{
    messages::{GameSubscription, MessageDestination, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::{DisconnectReason, WsTelemetry},
    ws_hub::WsHub,
    WebsocketData,
};
use crate::api::v1::auth::{decode::jwt_decode, jwt_secret::JwtSecret};
use crate::common::{ClientRequest, ExternalServerError, GameAction, ServerResult};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::{get_conn, models::User, DbPool};
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::SimpleUser;
use std::{
    cell::Cell,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Per-connection identity. Starts anonymous; an incoming
/// `ClientRequest::Auth(token)` swaps it in place for the decoded user.
/// Shared between `reader_task` (snapshots it on each frame) and
/// `DisconnectGuard` (reads it on drop so cleanup uses the post-auth
/// identity, not the anon UUID we started with).
#[derive(Clone)]
struct ConnIdentity {
    user_uid: Uuid,
    username: String,
    admin: bool,
    authed: bool,
}

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
    identity: Arc<Mutex<ConnIdentity>>,
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
        // Take the identity out of the mutex so the on_disconnect call
        // receives the username by value (matches the previous signature).
        // Replacing with an anon stub costs nothing — the guard is mid-drop.
        let identity = {
            let mut guard = self.identity.lock().expect("identity mutex poisoned");
            std::mem::replace(
                &mut *guard,
                ConnIdentity {
                    user_uid: Uuid::nil(),
                    username: String::new(),
                    admin: false,
                    authed: false,
                },
            )
        };
        self.hub
            .on_disconnect(self.socket_id, identity.user_uid, identity.username);
    }
}

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: AggregatedMessageStream,
    socket: SocketTx,
    hub: Arc<WsHub>,
    data: Arc<WebsocketData>,
    pool: DbPool,
    jwt_secret: Arc<JwtSecret>,
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

    let identity = Arc::new(Mutex::new(ConnIdentity {
        user_uid,
        username,
        admin,
        authed,
    }));

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
                    handle_binary(
                        &bytes,
                        &hub,
                        &socket,
                        &data,
                        &pool,
                        &jwt_secret,
                        &identity,
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
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<ConnIdentity>>,
) {
    data.telemetry.record_message_received(bytes.len());

    let request: Result<ClientRequest, _> = MsgpackSerdeCodec::decode(bytes);
    let Ok(request) = request else {
        return;
    };

    if let ClientRequest::Auth(token) = request {
        handle_auth(token, hub, socket, pool, jwt_secret.as_ref(), identity).await;
        return;
    }

    let snapshot = identity
        .lock()
        .expect("identity mutex poisoned")
        .clone();
    let ConnIdentity {
        user_uid: user_id,
        username,
        admin,
        authed,
    } = snapshot;

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
        username: username.clone(),
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

/// Process an `Auth(token)` frame. Decodes the JWT, looks up the user,
/// re-binds the socket from its anonymous identity to the real user in
/// the hub, and updates the shared `ConnIdentity`. Bad/expired tokens are
/// logged and dropped — the connection stays anonymous rather than
/// closing, so a stale token in localStorage doesn't kill the socket.
async fn handle_auth(
    token: String,
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    pool: &DbPool,
    jwt_secret: &JwtSecret,
    identity: &Arc<Mutex<ConnIdentity>>,
) {
    let Ok(sub) = jwt_decode(&token, &jwt_secret.decoding) else {
        log::debug!("WS auth: token decode failed");
        return;
    };
    let Ok(new_uid) = Uuid::parse_str(&sub) else {
        log::debug!("WS auth: sub is not a valid UUID");
        return;
    };
    let mut conn = match get_conn(pool).await {
        Ok(c) => c,
        Err(err) => {
            log::warn!("WS auth: DB pool unavailable: {err}");
            return;
        }
    };
    let Ok(user) = User::find_by_uuid(&new_uid, &mut conn).await else {
        log::debug!("WS auth: no user for uid {new_uid}");
        return;
    };

    // Snapshot+swap under the mutex so the disconnect guard can never
    // observe a partially-mutated identity.
    let (old_uid, old_username) = {
        let mut guard = identity.lock().expect("identity mutex poisoned");
        let old = std::mem::replace(
            &mut *guard,
            ConnIdentity {
                user_uid: new_uid,
                username: user.username.clone(),
                admin: user.admin,
                authed: true,
            },
        );
        (old.user_uid, old.username)
    };

    // Re-bind in the hub: drop the anon membership, register under the
    // real user with the same socket_id + tx so existing fanouts that
    // route by socket_id keep working.
    hub.on_disconnect(socket.socket_id, old_uid, old_username);
    Arc::clone(hub).on_connect(socket.socket_id, new_uid, user.username, socket.tx.clone());
}
