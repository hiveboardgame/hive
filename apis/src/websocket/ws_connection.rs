use super::{
    messages::{MessageDestination, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::DisconnectReason,
    ws_hub::WsHub,
    WebsocketData,
};
use crate::common::{ClientRequest, ExternalServerError, ServerResult};
use actix_ws::{Message, MessageStream, Session};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::DbPool;
use futures_util::StreamExt;
use indoc::printdoc;
use shared_types::SimpleUser;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: MessageStream,
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

    let mut last_hb = Instant::now();
    let mut hb_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    let mut reason = DisconnectReason::Close;

    loop {
        tokio::select! {
            _ = hb_interval.tick() => {
                if last_hb.elapsed() > CLIENT_TIMEOUT {
                    reason = DisconnectReason::Timeout;
                    break;
                }
                if session.ping(b"hi").await.is_err() {
                    reason = DisconnectReason::PingFail;
                    break;
                }
            }
            item = msg_stream.next() => match item {
                Some(Ok(Message::Ping(bytes))) => {
                    last_hb = Instant::now();
                    if session.pong(&bytes).await.is_err() {
                        reason = DisconnectReason::PingFail;
                        break;
                    }
                }
                Some(Ok(Message::Pong(_))) => {
                    last_hb = Instant::now();
                }
                Some(Ok(Message::Binary(bytes))) => {
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
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(Message::Continuation(_))) => {
                    reason = DisconnectReason::Continuation;
                    break;
                }
                Some(Ok(Message::Text(_))) | Some(Ok(Message::Nop)) => {}
                Some(Err(_)) => {
                    reason = DisconnectReason::StreamErr;
                    break;
                }
            }
        }
    }

    data.telemetry.record_disconnect(reason);
    hub.on_disconnect(socket.socket_id, user_uid, username);
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

    let user = SimpleUser {
        user_id,
        username: username.to_owned(),
        authed,
        admin,
    };
    let handler = RequestHandler::new(
        request.clone(),
        data.clone(),
        socket.clone(),
        user,
        pool.clone(),
    );

    match handler.handle().await {
        Ok(messages) => {
            for message in messages {
                let serialized = ServerResult::Ok(Box::new(message.message));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                    hub.dispatch(&message.destination, Bytes::from(serialized), Some(user_id))
                        .await;
                }
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
                    Some(user_id),
                )
                .await;
            }
        }
    }
}
