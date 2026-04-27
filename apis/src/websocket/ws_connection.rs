use super::{
    messages::{ClientActorMessage, Connect, Disconnect, MessageDestination, SocketTx},
    server_handlers::request_handler::{RequestHandler, RequestHandlerError},
    telemetry::DisconnectReason,
    WebsocketData,
};
use crate::common::{ClientRequest, ExternalServerError, ServerResult};
use actix::Addr;
use actix_ws::{Message, MessageStream, Session};
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

use crate::websocket::ws_server::WsServer;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn reader_task(
    mut session: Session,
    mut msg_stream: MessageStream,
    socket: SocketTx,
    srv: Addr<WsServer>,
    data: Arc<WebsocketData>,
    pool: DbPool,
    user_uid: Uuid,
    username: String,
    admin: bool,
    authed: bool,
) {
    if srv
        .send(Connect {
            socket: socket.clone(),
            game_id: String::from("lobby"),
            user_id: user_uid,
            username: username.clone(),
        })
        .await
        .is_err()
    {
        data.telemetry.record_handshake_fail();
        let _ = session.close(None).await;
        return;
    }

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
                        &srv,
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
    srv.do_send(Disconnect {
        socket_id: socket.socket_id,
        game_id: String::from("lobby"),
        user_id: user_uid,
        username,
    });
    let _ = session.close(None).await;
}

async fn handle_binary(
    bytes: &[u8],
    srv: &Addr<WsServer>,
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
                    srv.do_send(ClientActorMessage {
                        destination: message.destination,
                        serialized,
                        from: Some(user_id),
                    });
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
                srv.do_send(ClientActorMessage {
                    destination: MessageDestination::User(user_id),
                    serialized,
                    from: Some(user_id),
                });
            }
        }
    }
}
