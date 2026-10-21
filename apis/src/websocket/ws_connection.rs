use super::{
    lag_tracking::Probes,
    messages::{InternalServerMessage, MessageDestination, Outbound, SocketFormat, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::DisconnectReason,
    ws_hub::WsHub,
    WebsocketData,
};
use crate::{
    api::v1::auth::{decode::jwt_decode, jwt_secret::JwtSecret},
    common::{
        ChatSendError,
        ClientRequest,
        ExternalServerError,
        GameAction,
        ServerMessage,
        ServerResult,
        SubscriptionError,
        TournamentAction,
        TournamentUpdate,
    },
    responses::UserResponse,
};
use actix_ws::{AggregatedMessage, AggregatedMessageStream, Session};
use chrono::{DateTime, Utc};
use codee::{binary::MsgpackSerdeCodec, Decoder};
use db_lib::{game_command::ReceivedMove, get_conn, models::User, DbPool};
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::{ConversationKey, GameThread, SimpleUser};
use std::{
    cell::Cell,
    ops::ControlFlow,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const AUTH_REPLY_PATIENCE: Duration = Duration::from_secs(5);
const PROBE_INTERVAL: Duration = Duration::from_secs(1);
const PROBE_PATIENCE: Duration = Duration::from_millis(100);
// Jittered so a client's own periodic requests cannot line up with every probe and get all of
// its samples thrown out.
const PROBE_JITTER: f64 = 0.25;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
// Enough for a move to be read behind a few slow requests; each can hold a frame of up to 1 MiB.
const QUEUED_REQUESTS: usize = 4;

/// RAII cleanup so the WS subsystem stays consistent even if the reader loop
/// unwinds — without this, a panic in it (a poisoned lock) skips
/// `on_disconnect` and leaks the user in
/// `sessions`/membership, leaving `active_sockets`/`active_users` overcounted
/// for the lifetime of the process.
struct DisconnectGuard {
    hub: Arc<WsHub>,
    data: Arc<WebsocketData>,
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
        self.data.telemetry.record_disconnect(self.reason.get());
        self.data.socket_lags.disconnect(self.socket_id);
        // Read after any `Auth` swap, so cleanup unbinds the user the socket ended as.
        self.hub
            .on_disconnect(self.socket_id, current_identity(&self.identity));
    }
}

// A panic while holding the lock cannot leave a half-written `SimpleUser`: the only write
// is a whole-value swap.
fn current_identity(identity: &Mutex<SimpleUser>) -> SimpleUser {
    identity
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
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
    tcp_rtt: Option<Duration>,
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
    data.socket_lags.connect(socket.socket_id, tcp_rtt);
    let identity = Arc::new(Mutex::new(user));

    let guard = DisconnectGuard {
        hub: Arc::clone(&hub),
        data: Arc::clone(&data),
        socket_id: socket.socket_id,
        identity: Arc::clone(&identity),
        reason: Cell::new(DisconnectReason::Close),
    };

    let (requests, queued) = mpsc::channel(QUEUED_REQUESTS);
    let handling = actix_web::rt::spawn(handle_requests(
        queued,
        Deps {
            hub: Arc::clone(&hub),
            data: Arc::clone(&data),
            pool,
            jwt_secret,
        },
        socket.clone(),
        Arc::clone(&identity),
    ));

    let mut last_hb = Instant::now();
    let mut hb_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    let mut probes = Probes::default();
    let mut next_probe = tokio::time::Instant::now() + probe_delay();

    loop {
        tokio::select! {
            () = requests.closed() => {
                guard.set_reason(DisconnectReason::HandlerStopped);
                break;
            }
            _ = hb_interval.tick() => {
                if last_hb.elapsed() > CLIENT_TIMEOUT {
                    guard.set_reason(DisconnectReason::Timeout);
                    break;
                }
                if !send_probe(&mut session, &socket, &mut probes, HEARTBEAT_INTERVAL).await {
                    guard.set_reason(DisconnectReason::PingFail);
                    break;
                }
            }
            _ = tokio::time::sleep_until(next_probe) => {
                send_probe(&mut session, &socket, &mut probes, PROBE_PATIENCE).await;
                next_probe = tokio::time::Instant::now() + probe_delay();
            }
            item = msg_stream.next() => match item {
                Some(Ok(AggregatedMessage::Ping(bytes))) => {
                    last_hb = Instant::now();
                    let answered = session.pong(&bytes).await;
                    probes.reader_stalled(last_hb.elapsed());
                    if answered.is_err() {
                        guard.set_reason(DisconnectReason::PingFail);
                        break;
                    }
                }
                Some(Ok(AggregatedMessage::Pong(bytes))) => {
                    let now = Instant::now();
                    last_hb = now;
                    if let Some(rtt) = probes.round_trip(&bytes, now) {
                        data.socket_lags.record_round_trip(socket.socket_id, rtt);
                    }
                }
                Some(Ok(AggregatedMessage::Binary(bytes))) => {
                    last_hb = Instant::now();
                    data.telemetry.record_message_received(bytes.len());
                    let queued = match MsgpackSerdeCodec::decode(&bytes) {
                        Ok(request) => requests.send(received(request, &identity)).await.is_ok(),
                        Err(_) => true,
                    };
                    probes.reader_stalled(last_hb.elapsed());
                    if !queued {
                        guard.set_reason(DisconnectReason::HandlerStopped);
                        break;
                    }
                }
                Some(Ok(AggregatedMessage::Close(_))) => break,
                None => {
                    guard.set_reason(DisconnectReason::StreamErr);
                    break;
                }
                Some(Ok(AggregatedMessage::Text(text))) => {
                    last_hb = Instant::now();
                    data.telemetry.record_message_received(text.len());
                    let incoming = serde_json::from_str(&text).map_or_else(
                        |error| Incoming::Malformed(format!("Malformed request: {error}")),
                        |request| received(request, &identity),
                    );
                    let queued = requests.send(incoming).await.is_ok();
                    probes.reader_stalled(last_hb.elapsed());
                    if !queued {
                        guard.set_reason(DisconnectReason::HandlerStopped);
                        break;
                    }
                }
                Some(Err(_)) => {
                    guard.set_reason(DisconnectReason::StreamErr);
                    break;
                }
            }
        }
    }

    drop(requests);
    // Others see the socket gone at once, while the requests it already sent still run; the guard
    // then scrubs whatever they subscribed it to.
    hub.on_disconnect(socket.socket_id, current_identity(&identity));
    let _ = session.close(None).await;
    let _ = handling.await;
    drop(guard);
}

fn probe_delay() -> Duration {
    PROBE_INTERVAL.mul_f64(rand::random_range(1.0 - PROBE_JITTER..1.0 + PROBE_JITTER))
}

// The round trip starts once the ping is queued, so waiting for room in our send queue is not
// counted as the client's lag, and a ping behind a backlog there only proves the client is alive.
// Backlog further down is out of sight, but the game's updates wait behind it too, and the TCP cap
// bounds it.
async fn send_probe(
    session: &mut Session,
    socket: &SocketTx,
    probes: &mut Probes,
    patience: Duration,
) -> bool {
    let nonce = rand::random::<u64>();
    let backlogged = socket.tx.capacity() < socket.tx.max_capacity();
    let started = Instant::now();
    let queued = tokio::time::timeout(patience, session.ping(&nonce.to_be_bytes())).await;
    probes.reader_stalled(started.elapsed());
    if !matches!(queued, Ok(Ok(()))) {
        return false;
    }
    if !backlogged {
        probes.sent(nonce, Instant::now());
    }
    true
}

enum Incoming {
    Request {
        request: ClientRequest,
        received_move: Option<ReceivedMove>,
    },
    Malformed(String),
}

// A move is stamped and recorded as it is read, so time it then waits behind slower requests is
// not charged to the mover.
fn received(request: ClientRequest, identity: &Mutex<SimpleUser>) -> Incoming {
    let received_move = match &request {
        ClientRequest::Game {
            game_id,
            action: GameAction::Turn(_) | GameAction::Play(_) | GameAction::PlayAt { .. },
        } => {
            let user = current_identity(identity);
            user.authed
                .then(|| ReceivedMove::arriving(&game_id.0, user.user_id))
        }
        _ => None,
    };
    Incoming::Request {
        request,
        received_move,
    }
}

async fn handle_requests(
    mut queued: mpsc::Receiver<Incoming>,
    deps: Deps,
    socket: SocketTx,
    identity: Arc<Mutex<SimpleUser>>,
) {
    while let Some(incoming) = queued.recv().await {
        let flow = match incoming {
            Incoming::Request {
                request,
                received_move,
            } => {
                let move_arrival = received_move.as_ref().map(ReceivedMove::arrived_at);
                handle_request(request, move_arrival, &deps, &socket, &identity).await
            }
            Incoming::Malformed(reason) => {
                // Only bots send text frames, and they are writing the JSON by hand: tell them
                // what is wrong instead of leaving them waiting for a reply that will never come.
                let message = ServerResult::Err(ExternalServerError::Request { reason });
                deps.hub
                    .dispatch_out(
                        &MessageDestination::Direct(socket.clone()),
                        &mut Outbound::typed(&message),
                    )
                    .await;
                ControlFlow::Continue(())
            }
        };
        if flow.is_break() {
            return;
        }
    }
}

async fn handle_request(
    request: ClientRequest,
    move_arrival: Option<DateTime<Utc>>,
    deps: &Deps,
    socket: &SocketTx,
    identity: &Arc<Mutex<SimpleUser>>,
) -> ControlFlow<()> {
    let Deps {
        hub,
        data,
        pool,
        jwt_secret,
    } = deps;
    if let ClientRequest::Auth(token) = request {
        // Boxed so the login, profile build and lobby snapshot behind it stay out of the
        // future every *other* frame — an ordinary move included — is dispatched through.
        return Box::pin(handle_auth(&token, hub, socket, pool, jwt_secret, identity)).await;
    }

    let user = current_identity(identity);

    // Unwatch needs hub access and no DB — handle it here before RequestHandler.
    if let ClientRequest::Game {
        ref game_id,
        action: GameAction::Unwatch,
    } = request
    {
        hub.unsubscribe_game(user.user_id, socket.socket_id, game_id);
        return ControlFlow::Continue(());
    }

    let handler = RequestHandler::new(
        request.clone(),
        move_arrival,
        data.clone(),
        hub.clone(),
        socket.clone(),
        user.clone(),
        pool.clone(),
    );

    match handler.handle().await {
        Ok(mut output) => {
            let request_error = output.request_error.take();
            hub.dispatch_handler_output(output).await;
            if let Some(error) = request_error {
                dispatch_request_error(
                    hub,
                    socket,
                    &request,
                    &user,
                    RequestHandlerError::InternalError(error),
                )
                .await;
            }
        }
        Err(err) => {
            dispatch_request_error(hub, socket, &request, &user, err).await;
        }
    }
    ControlFlow::Continue(())
}

async fn dispatch_request_error(
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    request: &ClientRequest,
    user: &SimpleUser,
    err: RequestHandlerError,
) {
    if matches!(err, RequestHandlerError::RateLimited(_)) {
        hub.data.telemetry.record_chat_rate_limit_rejection();
    }
    if let Some(reason) = rejection(request, &err) {
        log::warn!(
            "Rejected {} from {} {}: {reason}",
            request_log_summary(request),
            user.username,
            user.user_id
        );
    } else if should_log_request_error(&err) {
        let request_summary = request_log_summary(request);
        printdoc! {r#"
            -----------------ERROR-----------------
              Request: {}
              Error:   {:#?}
              User:    {} {}
            ------------------END------------------
            "#,
            request_summary, err, user.username, user.user_id
        };
    }
    let failure = match request {
        ClientRequest::Tournament(TournamentAction::Create(_)) => {
            Some(TournamentUpdate::CreationFailed)
        }
        ClientRequest::Tournament(TournamentAction::InvitationCreate(
            tournament_id,
            invitee_id,
        )) => Some(TournamentUpdate::InvitationFailed {
            tournament_id: tournament_id.clone(),
            invitee_id: *invitee_id,
        }),
        _ => None,
    };
    if let Some(update) = failure {
        hub.dispatch_message(InternalServerMessage {
            destination: MessageDestination::Direct(socket.clone()),
            message: ServerMessage::Tournament(update),
        })
        .await;
    }
    let message = ServerResult::Err(for_format(
        external_server_error(request, &err),
        socket.format,
    ));
    hub.dispatch_out(
        &MessageDestination::Direct(socket.clone()),
        &mut Outbound::typed(&message),
    )
    .await;
}

/// Requests a bot makes, where it needs to know why it was refused. Other requests keep the
/// generic reason, because their internal errors can carry private tournament details.
fn rejection(request: &ClientRequest, error: &RequestHandlerError) -> Option<String> {
    match request {
        ClientRequest::Game { .. }
        | ClientRequest::Challenge(_)
        | ClientRequest::GetGame(_)
        | ClientRequest::GetPendingGames
        | ClientRequest::GetOngoingGames
        | ClientRequest::GetUser(_)
        | ClientRequest::GetUsername(_) => error.rejection_reason(),
        _ => None,
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
                RequestHandlerError::ChatMuted => ChatSendError::Muted,
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
                RequestHandlerError::TournamentNameTaken => ChatSendError::Unavailable,
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
                | RequestHandlerError::ChatMuted
                | RequestHandlerError::ChatClientIdConflict => SubscriptionError::Unavailable,
                RequestHandlerError::TournamentNameTaken => SubscriptionError::Unavailable,
            };
            ExternalServerError::ChatSubscribe {
                attempt: subscription.clone(),
                error: subscription_error,
            }
        }
        ClientRequest::Game { game_id, .. } => ExternalServerError::Game {
            game_id: game_id.clone(),
            reason: rejection(request, error).unwrap_or_else(|| error.user_safe_reason()),
        },
        _ => ExternalServerError::Request {
            reason: rejection(request, error).unwrap_or_else(|| error.user_safe_reason()),
        },
    }
}

// Tabs keep the bundle they loaded across a deploy, and one from before `Game` existed cannot
// decode it. Browsers have no use for the game id.
fn for_format(error: ExternalServerError, format: SocketFormat) -> ExternalServerError {
    match (format, error) {
        (SocketFormat::Msgpack, ExternalServerError::Game { reason, .. }) => {
            ExternalServerError::Request { reason }
        }
        (_, error) => error,
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
            | RequestHandlerError::ChatMuted
    ) && !err.is_realtime_pause()
}

// A bot waits for the answer to its `Auth` before anything else, so a full queue delays the
// answer instead of losing it, and a socket that never makes room is hung up on: the bot
// reconnects rather than waiting forever.
async fn auth_reply(
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    message: ServerMessage,
) -> ControlFlow<()> {
    let message = ServerResult::Ok(Box::new(message));
    if hub
        .send_direct_waiting(socket, &mut Outbound::typed(&message), AUTH_REPLY_PATIENCE)
        .await
    {
        ControlFlow::Continue(())
    } else {
        ControlFlow::Break(())
    }
}

async fn auth_failed(hub: &Arc<WsHub>, socket: &SocketTx) -> ControlFlow<()> {
    auth_reply(hub, socket, ServerMessage::Error("Auth failed".to_string())).await
}

async fn handle_auth(
    token: &str,
    hub: &Arc<WsHub>,
    socket: &SocketTx,
    pool: &DbPool,
    jwt_secret: &Arc<JwtSecret>,
    identity: &Arc<Mutex<SimpleUser>>,
) -> ControlFlow<()> {
    // A second `Auth` would re-run the Offline/Online broadcast and reset the socket's rate
    // limits, so a socket authenticates once.
    if current_identity(identity).authed {
        return auth_reply(
            hub,
            socket,
            ServerMessage::Error("Already authenticated".to_string()),
        )
        .await;
    }
    if !hub.allow_auth_attempt(socket.socket_id) {
        return auth_reply(
            hub,
            socket,
            ServerMessage::Error("Too many Auth attempts".to_string()),
        )
        .await;
    }

    let Ok(email) = jwt_decode(token, &jwt_secret.decoding) else {
        return auth_failed(hub, socket).await;
    };
    let Some((user, user_response)) = find_bot(&email, pool).await else {
        return auth_failed(hub, socket).await;
    };

    let authenticated = SimpleUser {
        user_id: user.id,
        username: user.username.clone(),
        admin: user.admin,
        authed: true,
        bot: user.bot,
    };

    // The ack goes out before the re-bind so it precedes every message the new
    // registration produces, including our own Online. The identity changes only once the ack
    // is queued, so a socket hung up on for want of room is cleaned up as the anonymous one it
    // still is.
    auth_reply(hub, socket, ServerMessage::Authenticated(user_response)).await?;
    let previous = {
        let mut guard = identity.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::replace(&mut *guard, authenticated.clone())
    };
    Arc::clone(hub).on_authenticate(socket, previous, authenticated);
    ControlFlow::Continue(())
}

// The connection goes back to the pool here, before any reply waits for room on the socket.
async fn find_bot(email: &str, pool: &DbPool) -> Option<(User, UserResponse)> {
    let mut conn = get_conn(pool).await.ok()?;
    let user = User::find_for_login(email, &mut conn).await.ok()?;
    // The same gate the HTTP bot API applies, so the socket cannot become a second way in
    // for a token the HTTP side would refuse.
    if !user.bot {
        return None;
    }
    // Built before the identity swap: a failure here must leave the socket anonymous,
    // matching the "Auth failed" we send back.
    let user_response = UserResponse::from_model(&user, &mut conn).await.ok()?;
    Some((user, user_response))
}

#[cfg(test)]
mod tests {
    use super::{external_server_error, for_format, should_log_request_error};
    use crate::{
        common::{
            ChatSendError,
            ChatSendRequest,
            ClientRequest,
            ExternalServerError,
            SubscriptionAttempt,
            SubscriptionError,
            UiError,
        },
        websocket::{
            messages::{AuthError, SocketFormat},
            server_handlers::{chat::limits::ChatLimitError, request_handler::RequestHandlerError},
            RealtimeGamesPaused,
        },
    };
    use db_lib::db_error::DbError;
    use shared_types::{ConversationKey, GameId, TournamentId};
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn internal_database_details_do_not_cross_the_websocket_boundary() {
        let request = ClientRequest::TournamentWatch(TournamentId("private-details".to_string()));
        let first = RequestHandlerError::InternalError(
            DbError::InvalidInput {
                info: "internal tournament context".to_string(),
                error: "internal pairing failure".to_string(),
            }
            .into(),
        );
        let second = RequestHandlerError::InternalError(
            DbError::InvalidInput {
                info: "different internal context".to_string(),
                error: "different internal failure".to_string(),
            }
            .into(),
        );
        let response = external_server_error(&request, &first);
        assert_eq!(response, external_server_error(&request, &second));
        let ExternalServerError::Request { reason } = response else {
            panic!("internal tournament errors use the generic request response");
        };
        assert!(!reason.contains("internal tournament context"));
        assert!(!reason.contains("internal pairing failure"));
    }

    #[test]
    fn a_refused_move_names_its_game_and_the_reason() {
        let game_id = GameId("bot-game".to_string());
        let request = ClientRequest::Game {
            game_id: game_id.clone(),
            action: crate::common::GameAction::Play("wA1 -bQ".to_string()),
        };
        let refused = RequestHandlerError::InternalError(
            DbError::InvalidAction {
                info: "It is not this player's turn".to_string(),
            }
            .into(),
        );

        assert_eq!(
            external_server_error(&request, &refused),
            ExternalServerError::Game {
                game_id,
                reason: "Invalid action: It is not this player's turn".to_string(),
            }
        );
    }

    #[test]
    fn a_browser_gets_a_refused_move_in_the_shape_old_tabs_decode() {
        let refused = ExternalServerError::Game {
            game_id: GameId("browser-game".to_string()),
            reason: "Invalid action: It is not this player's turn".to_string(),
        };

        assert_eq!(
            for_format(refused.clone(), SocketFormat::Msgpack),
            ExternalServerError::Request {
                reason: "Invalid action: It is not this player's turn".to_string(),
            }
        );
        assert_eq!(for_format(refused.clone(), SocketFormat::Json), refused);
    }

    #[test]
    fn an_internal_failure_behind_a_move_stays_generic() {
        let request = ClientRequest::Game {
            game_id: GameId("bot-game".to_string()),
            action: crate::common::GameAction::Play("wA1 -bQ".to_string()),
        };
        let internal = RequestHandlerError::InternalError(
            DbError::InternalError {
                reason: "connection reset by peer".to_string(),
            }
            .into(),
        );

        let ExternalServerError::Game { reason, .. } = external_server_error(&request, &internal)
        else {
            panic!("a game request fails with a game error");
        };
        assert_eq!(reason, UiError::RequestUnavailable.encode());
    }

    #[test]
    fn a_paused_realtime_refusal_is_explained_and_not_logged() {
        let request = ClientRequest::Challenge(crate::common::ChallengeAction::Accept(
            shared_types::ChallengeId("realtime".to_string()),
        ));
        let paused = RequestHandlerError::InternalError(
            anyhow::Error::new(RealtimeGamesPaused).context("accepting challenge"),
        );

        assert_eq!(
            external_server_error(&request, &paused),
            ExternalServerError::Request {
                reason: UiError::RealtimeGamesPaused.encode(),
            }
        );
        assert!(!should_log_request_error(&paused));
    }

    #[test]
    fn bad_notation_is_reported_as_such() {
        let request = ClientRequest::Game {
            game_id: GameId("bot-game".to_string()),
            action: crate::common::GameAction::Play("wA1".to_string()),
        };
        let unparsable = RequestHandlerError::InternalError(
            hive_lib::GameError::ParsingError {
                found: "wA1".to_string(),
                typ: "move, expected 'piece position'".to_string(),
            }
            .into(),
        );

        let ExternalServerError::Game { reason, .. } = external_server_error(&request, &unparsable)
        else {
            panic!("a game request fails with a game error");
        };
        assert!(reason.contains("wA1"), "{reason}");
    }

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

    /// Acked first, so a bot learns its own uid before anything else; no lobby snapshot
    /// follows, the bot asks for its games; and a second `Auth` on the same socket is refused
    /// rather than re-running the rebind.
    #[tokio::test(flavor = "multi_thread")]
    async fn auth_acks_once_and_sends_no_snapshot() {
        use crate::{
            api::v1::auth::{
                encode::{jwt_encode, Bot},
                jwt_secret::JwtSecret,
            },
            common::{ServerMessage, ServerResult},
            responses::UserResponse,
            websocket::{
                messages::{SocketFormat, SocketTx},
                test_db::test_db,
                ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY},
                WebsocketData,
            },
        };
        use db_lib::{
            get_conn,
            models::{NewUser, User},
        };
        use shared_types::SimpleUser;
        use std::sync::{Arc, Mutex};

        let db = test_db().await;
        let pool = db.pool.clone();
        // The rebind spawns its loader with `actix_rt::spawn`, which needs a local task set.
        tokio::task::LocalSet::new()
            .run_until(async move {
                let mut conn = get_conn(&pool).await.expect("test database connection");
                let mut new_user = NewUser::new("ackbot", "password", "ackbot@example.com")
                    .expect("valid bot user");
                new_user.bot = true;
                let user = User::create(new_user, &mut conn)
                    .await
                    .expect("bot user is created");

                let jwt_secret = Arc::new(JwtSecret::new("test-secret".to_string()));
                let token = jwt_encode(
                    Bot {
                        email: "ackbot@example.com".to_string(),
                        password: "password".to_string(),
                    },
                    &jwt_secret.encoding,
                )
                .expect("token encodes");

                let hub = WsHub::new(Arc::new(WebsocketData::default()), pool.clone());
                let (tx, mut rx) = tokio::sync::mpsc::channel(SOCKET_BUFFER_CAPACITY);
                let socket = SocketTx {
                    socket_id: Uuid::new_v4(),
                    format: SocketFormat::Json,
                    tx,
                };
                let identity = Arc::new(Mutex::new(SimpleUser {
                    user_id: Uuid::new_v4(),
                    username: "anonymous".to_string(),
                    admin: false,
                    authed: false,
                    bot: false,
                }));

                assert!(
                    super::handle_auth(&token, &hub, &socket, &pool, &jwt_secret, &identity)
                        .await
                        .is_continue()
                );
                assert!(super::current_identity(&identity).authed);
                assert!(super::current_identity(&identity).bot);
                assert!(
                    super::handle_auth(&token, &hub, &socket, &pool, &jwt_secret, &identity)
                        .await
                        .is_continue()
                );

                let mut frames = Vec::new();
                while let Ok(Some(bytes)) =
                    tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
                {
                    let decoded: ServerResult =
                        serde_json::from_slice(&bytes).expect("a json socket is sent JSON");
                    let ServerResult::Ok(message) = decoded else {
                        panic!("no frame here is an error envelope");
                    };
                    frames.push(*message);
                }

                let Some(ServerMessage::Authenticated(UserResponse { uid, .. })) = frames.first()
                else {
                    panic!("a bot learns it is authenticated before any other frame: {frames:?}");
                };
                assert_eq!(*uid, user.id);
                assert!(
                    !frames
                        .iter()
                        .any(|frame| matches!(frame, ServerMessage::LobbySnapshot(_))),
                    "Auth must not push a lobby snapshot: {frames:?}"
                );
                assert!(
                    frames.iter().any(|frame| matches!(
                        frame,
                        ServerMessage::Error(reason) if reason == "Already authenticated"
                    )),
                    "a second Auth on the same socket is refused: {frames:?}"
                );
            })
            .await;
    }

    /// A bot waits for this ack before anything else and cannot ask twice, so a backlog delays
    /// it instead of losing it, and a socket that cannot take it is hung up on still anonymous.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_auth_ack_waits_out_a_backlog_and_never_goes_missing() {
        use crate::{
            api::v1::auth::{
                encode::{jwt_encode, Bot},
                jwt_secret::JwtSecret,
            },
            common::{ServerMessage, ServerResult},
            responses::UserResponse,
            websocket::{
                messages::{SocketFormat, SocketTx},
                test_db::test_db,
                ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY},
                WebsocketData,
            },
        };
        use bytes::Bytes;
        use db_lib::{
            get_conn,
            models::{NewUser, User},
        };
        use shared_types::SimpleUser;
        use std::{
            sync::{Arc, Mutex},
            time::Duration,
        };
        use tokio::sync::mpsc;

        let db = test_db().await;
        let pool = db.pool.clone();
        tokio::task::LocalSet::new()
            .run_until(async move {
                let mut conn = get_conn(&pool).await.expect("test database connection");
                let mut new_user = NewUser::new("backlogbot", "password", "backlogbot@example.com")
                    .expect("valid bot user");
                new_user.bot = true;
                let user = User::create(new_user, &mut conn)
                    .await
                    .expect("bot user is created");
                drop(conn);
                let jwt_secret = Arc::new(JwtSecret::new("test-secret".to_string()));
                let token = jwt_encode(
                    Bot {
                        email: "backlogbot@example.com".to_string(),
                        password: "password".to_string(),
                    },
                    &jwt_secret.encoding,
                )
                .expect("token encodes");
                let hub = WsHub::new(Arc::new(WebsocketData::default()), pool.clone());
                let anonymous = || {
                    Arc::new(Mutex::new(SimpleUser {
                        user_id: Uuid::new_v4(),
                        username: "anonymous".to_string(),
                        admin: false,
                        authed: false,
                        bot: false,
                    }))
                };

                let (tx, mut rx) = mpsc::channel(SOCKET_BUFFER_CAPACITY);
                for _ in 0..SOCKET_BUFFER_CAPACITY {
                    tx.try_send(Bytes::from_static(b"backlog"))
                        .expect("the queue has room");
                }
                let backlogged = SocketTx {
                    socket_id: Uuid::new_v4(),
                    format: SocketFormat::Json,
                    tx,
                };
                let waiting_pool = pool.clone();
                let drained = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let state = waiting_pool.state();
                    let held_while_waiting = state.connections - state.idle_connections;
                    let mut frames = Vec::new();
                    while let Ok(Some(bytes)) =
                        tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
                    {
                        frames.push(bytes);
                    }
                    (held_while_waiting, frames)
                });
                let identity = anonymous();
                assert!(super::handle_auth(
                    &token,
                    &hub,
                    &backlogged,
                    &pool,
                    &jwt_secret,
                    &identity
                )
                .await
                .is_continue());
                assert!(super::current_identity(&identity).authed);
                let (held_while_waiting, frames) = drained.await.expect("the drain finishes");
                assert_eq!(
                    held_while_waiting, 0,
                    "a socket with no room must not keep a database connection from the pool"
                );
                let ack: ServerResult = serde_json::from_slice(&frames[SOCKET_BUFFER_CAPACITY])
                    .expect("the ack follows the backlog");
                let ServerResult::Ok(ack) = ack else {
                    panic!("the ack is not an error envelope");
                };
                let ServerMessage::Authenticated(UserResponse { uid, .. }) = *ack else {
                    panic!("the frame after the backlog is the ack: {ack:?}");
                };
                assert_eq!(uid, user.id);

                let (tx, rx) = mpsc::channel(SOCKET_BUFFER_CAPACITY);
                drop(rx);
                let gone = SocketTx {
                    socket_id: Uuid::new_v4(),
                    format: SocketFormat::Json,
                    tx,
                };
                let identity = anonymous();
                assert!(
                    super::handle_auth(&token, &hub, &gone, &pool, &jwt_secret, &identity)
                        .await
                        .is_break()
                );
                assert!(!super::current_identity(&identity).authed);
            })
            .await;
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
            bot: false,
        };
        let identity = Arc::new(Mutex::new(anonymous.clone()));
        let jwt_secret = Arc::new(JwtSecret::new("test-secret".to_string()));

        // Decoding fails before any database access, so the unreachable pool is never touched.
        assert!(
            super::handle_auth("not-a-jwt", &hub, &socket, &pool, &jwt_secret, &identity)
                .await
                .is_continue()
        );

        let after = identity.lock().expect("identity mutex poisoned").clone();
        assert!(!after.authed);
        assert_eq!(after.user_id, anonymous.user_id);
    }

    /// The frame dispatcher is the parent frame of every handler chain, so its size is
    /// paid by requests that never touch the branch responsible for it — `handle_auth`
    /// inlines a login, a profile build and a whole lobby snapshot into the same future an
    /// ordinary move is dispatched through. Debug builds neither pack async state machines
    /// nor elide the temporaries used to build them, so this is what decides whether the
    /// chain below still fits an Actix worker stack.
    #[tokio::test]
    async fn the_frame_dispatch_future_fits_a_worker_stack() {
        use crate::{
            api::v1::auth::jwt_secret::JwtSecret,
            common::GameAction,
            websocket::{
                messages::{SocketFormat, SocketTx},
                ws_hub::WsHub,
                WebsocketData,
            },
        };
        use shared_types::SimpleUser;
        use std::sync::{Arc, Mutex};

        const DISPATCH_FUTURE_BUDGET: usize = 16 * 1024;

        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        let data = Arc::new(WebsocketData::default());
        let hub = WsHub::new(data.clone(), pool.clone());
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let socket = SocketTx {
            socket_id: Uuid::new_v4(),
            format: SocketFormat::Msgpack,
            tx,
        };
        let identity = Arc::new(Mutex::new(SimpleUser {
            user_id: Uuid::nil(),
            username: "anonymous".to_string(),
            admin: false,
            authed: false,
            bot: false,
        }));
        let deps = super::Deps {
            hub,
            data,
            pool,
            jwt_secret: Arc::new(JwtSecret::new("test-secret".to_string())),
        };
        let request = ClientRequest::Game {
            game_id: GameId("some-game".to_string()),
            action: GameAction::Play("wA1 -bQ".to_string()),
        };

        let size = std::mem::size_of_val(&super::handle_request(
            request, None, &deps, &socket, &identity,
        ));

        println!("handle_request future: {size} bytes of {DISPATCH_FUTURE_BUDGET}");
        assert!(
            size <= DISPATCH_FUTURE_BUDGET,
            "handle_request is a {size} byte future, budget is {DISPATCH_FUTURE_BUDGET}; \
             box the branch futures that grew it"
        );
    }

    async fn unreachable_pool() -> db_lib::DbPool {
        db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting")
    }

    // A real socket, so frames go through actix-ws: unit tests cannot see the reader loop's wiring.
    async fn hub_without_database() -> std::sync::Arc<crate::websocket::WsHub> {
        crate::websocket::WsHub::new(
            std::sync::Arc::new(crate::websocket::WebsocketData::default()),
            unreachable_pool().await,
        )
    }

    async fn serve_reader(
        socket_id: Uuid,
        user: shared_types::SimpleUser,
        hub: std::sync::Arc<crate::websocket::WsHub>,
        tcp_rtt: Option<Duration>,
    ) -> std::net::SocketAddr {
        use super::{reader_task, Deps};
        use crate::{
            api::v1::auth::jwt_secret::JwtSecret,
            websocket::messages::{SocketFormat, SocketTx},
        };
        use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
        use std::sync::Arc;

        let (data, pool) = (Arc::clone(&hub.data), hub.pool.clone());
        let server = HttpServer::new(move || {
            let (hub, data, pool, user) = (hub.clone(), data.clone(), pool.clone(), user.clone());
            App::new().route(
                "/ws/",
                web::get().to(move |req: HttpRequest, body: web::Payload| {
                    let (hub, data, pool, user) =
                        (hub.clone(), data.clone(), pool.clone(), user.clone());
                    async move {
                        let (response, session, stream) = actix_ws::handle(&req, body)?;
                        let (tx, outbound) = tokio::sync::mpsc::channel(8);
                        actix_web::rt::spawn(async move {
                            let _outbound = outbound;
                            reader_task(
                                session,
                                stream.aggregate_continuations(),
                                SocketTx {
                                    socket_id,
                                    format: SocketFormat::Json,
                                    tx,
                                },
                                Deps {
                                    hub,
                                    data,
                                    pool,
                                    jwt_secret: Arc::new(JwtSecret::new("test-secret".to_string())),
                                },
                                user,
                                tcp_rtt,
                            )
                            .await;
                        });
                        Ok::<HttpResponse, actix_web::Error>(response)
                    }
                }),
            )
        })
        .workers(1)
        .bind(("127.0.0.1", 0))
        .expect("binds a free port");
        let addr = server.addrs()[0];
        actix_web::rt::spawn(server.run());
        addr
    }

    fn upgraded(addr: std::net::SocketAddr) -> std::net::TcpStream {
        use std::io::{Read, Write};

        let mut stream = std::net::TcpStream::connect(addr).expect("connects");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("sets a read timeout");
        write!(
            stream,
            "GET /ws/ HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        )
        .expect("sends the upgrade");
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).expect("reads the upgrade");
            head.push(byte[0]);
        }
        assert!(
            head.starts_with(b"HTTP/1.1 101"),
            "{:?}",
            String::from_utf8_lossy(&head)
        );
        stream
    }

    fn masked(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x80 | opcode, 0x80 | payload.len() as u8, 0, 0, 0, 0];
        frame.extend_from_slice(payload);
        frame
    }

    fn await_pong(stream: &mut std::net::TcpStream, payload: &[u8]) {
        use std::io::Read;

        loop {
            let mut header = [0u8; 2];
            stream
                .read_exact(&mut header)
                .expect("reads a frame header");
            let mut frame = vec![0u8; usize::from(header[1] & 0x7F)];
            stream
                .read_exact(&mut frame)
                .expect("reads a frame payload");
            if header[0] == 0x8A && frame == payload {
                return;
            }
        }
    }

    fn anonymous() -> shared_types::SimpleUser {
        shared_types::SimpleUser {
            user_id: Uuid::new_v4(),
            username: "anonymous".to_string(),
            admin: false,
            authed: false,
            bot: false,
        }
    }

    #[actix_web::test]
    async fn protocol_pongs_measure_their_socket_until_it_closes() {
        use std::{io::Write, sync::Arc};

        const NETWORK_DELAY: Duration = Duration::from_millis(100);
        const TCP_RTT: Duration = Duration::from_millis(40);

        let hub = hub_without_database().await;
        let data = Arc::clone(&hub.data);
        let socket_id = Uuid::new_v4();
        let addr = serve_reader(socket_id, anonymous(), hub, Some(TCP_RTT)).await;

        let client = tokio::task::spawn_blocking(move || {
            use std::io::Read;

            let mut stream = upgraded(addr);
            let mut ping = [0u8; 10];
            stream.read_exact(&mut ping).expect("reads the first probe");
            assert_eq!(&ping[..2], &[0x89, 8], "an unmasked 8-byte ping frame");
            std::thread::sleep(NETWORK_DELAY);
            stream
                .write_all(&masked(0xA, &ping[2..]))
                .expect("answers the probe");
            stream
                .write_all(&masked(0xA, &ping[2..]))
                .expect("answers it twice");
            stream
                .write_all(&masked(0xA, b"forged!!"))
                .expect("answers no probe");
            // The reader handles frames in order, so the answer to our own ping proves the
            // pongs before it were handled.
            stream
                .write_all(&masked(0x9, b"sync"))
                .expect("pings the server");
            await_pong(&mut stream, b"sync");
            stream
        })
        .await
        .expect("the client thread finishes");

        let measured = data.socket_lags.lag(socket_id);
        let delay = NETWORK_DELAY.as_secs_f64() * 1000.0;
        let estimate = measured.estimate_ms.expect("the pong was measured");
        assert!(
            (delay..1000.0).contains(&estimate),
            "{estimate} ms for a {delay} ms delay"
        );
        assert_eq!(
            data.socket_lags.sample_count(socket_id),
            1,
            "the duplicate and the forged pong are not samples"
        );
        assert_eq!(measured.cap_ms, 65.0);
        assert_eq!(measured.compensable_ms(), estimate.min(65.0));

        drop(client);
        let mut forgotten = false;
        for _ in 0..100 {
            if data.socket_lags.lag(socket_id).cap_ms == 1000.0 {
                forgotten = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(forgotten, "a closed socket leaves no lag state behind");
    }

    // The unreachable pool keeps it waiting for a connection far longer than the client's read
    // timeout.
    fn join_waiting_for_the_pool() -> Vec<u8> {
        serde_json::to_vec(&ClientRequest::Game {
            game_id: GameId("waiting-for-the-pool".to_string()),
            action: crate::common::GameAction::Join,
        })
        .expect("the request encodes")
    }

    #[actix_web::test]
    async fn frames_behind_a_request_still_being_handled_are_read() {
        use std::io::Write;

        let addr = serve_reader(
            Uuid::new_v4(),
            anonymous(),
            hub_without_database().await,
            None,
        )
        .await;

        tokio::task::spawn_blocking(move || {
            let mut stream = upgraded(addr);
            stream
                .write_all(&masked(0x1, &join_waiting_for_the_pool()))
                .expect("sends a request");
            stream
                .write_all(&masked(0x9, b"behind"))
                .expect("pings the server");
            await_pong(&mut stream, b"behind");
        })
        .await
        .expect("the ping is answered while the request still waits");
    }

    #[actix_web::test]
    async fn a_closed_socket_is_gone_while_its_requests_still_wait() {
        use std::{io::Write, sync::Arc};

        let player = shared_types::SimpleUser {
            authed: true,
            ..anonymous()
        };
        let user_id = player.user_id;
        let hub = hub_without_database().await;
        let addr = serve_reader(Uuid::new_v4(), player, Arc::clone(&hub), None).await;

        let client_hub = Arc::clone(&hub);
        tokio::task::spawn_blocking(move || {
            let mut stream = upgraded(addr);
            stream
                .write_all(&masked(0x1, &join_waiting_for_the_pool()))
                .expect("sends a request");
            stream
                .write_all(&masked(0x9, b"queued"))
                .expect("pings the server");
            await_pong(&mut stream, b"queued");
            assert_eq!(client_hub.socket_count(user_id), 1);
            stream.write_all(&masked(0x8, &[])).expect("closes");
        })
        .await
        .expect("the client thread finishes");

        let mut gone = false;
        for _ in 0..100 {
            if hub.socket_count(user_id) == 0 {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            gone,
            "the socket must not stay registered behind a waiting request"
        );
    }

    #[test]
    fn a_move_is_stamped_as_it_is_read_and_only_for_a_signed_in_player() {
        use super::{received, Incoming};
        use crate::common::GameAction;
        use chrono::Utc;
        use std::sync::Mutex;

        let player = shared_types::SimpleUser {
            authed: true,
            ..anonymous()
        };
        let game_id = GameId("read-time".to_string());
        let play = || ClientRequest::Game {
            game_id: game_id.clone(),
            action: GameAction::Play("wA1".to_string()),
        };

        let before = Utc::now();
        let Incoming::Request {
            received_move: Some(received_move),
            ..
        } = received(play(), &Mutex::new(player.clone()))
        else {
            panic!("a move is recorded as it is read");
        };
        assert!((before..=Utc::now()).contains(&received_move.arrived_at()));

        let join = ClientRequest::Game {
            game_id: game_id.clone(),
            action: GameAction::Join,
        };
        for (request, user) in [(play(), anonymous()), (join, player)] {
            assert!(matches!(
                received(request, &Mutex::new(user)),
                Incoming::Request {
                    received_move: None,
                    ..
                }
            ));
        }
    }
}
