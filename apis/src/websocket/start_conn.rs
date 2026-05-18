use std::sync::Arc;

use crate::api::v1::auth::jwt_secret::JwtSecret;
use crate::websocket::{
    messages::SocketTx,
    ws_connection::reader_task,
    ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY},
    WebsocketData,
};
use actix_identity::Identity;
use actix_web::{
    get,
    web::{Data, Payload},
    Error,
    HttpRequest,
    HttpResponse,
};
use bytes::Bytes;
use db_lib::{get_conn, models::User, DbPool};
use tokio::sync::mpsc;
use uuid::Uuid;

#[get("/ws/")]
pub async fn start_connection(
    req: HttpRequest,
    body: Payload,
    hub: Data<Arc<WsHub>>,
    pool: Data<DbPool>,
    identity: Option<Identity>,
    data: Data<WebsocketData>,
    jwt_secret: Data<JwtSecret>,
) -> Result<HttpResponse, Error> {
    let (user_uid, username, admin, authed) = resolve_identity(identity, &pool).await;

    let ws_result = actix_ws::handle(&req, body);
    if ws_result.is_err() {
        data.telemetry.record_handshake_fail();
    }
    let (response, session, msg_stream) = ws_result?;
    // Auto-assemble fragmented frames. Without this, a fragmented Binary
    // message arrives as Binary(first) + Continuation(...) and the reader
    // would have to handle Continuation explicitly. The default 1 MiB cap
    // is well above any legitimate msgpack frame in this app.
    let msg_stream = msg_stream.aggregate_continuations();
    data.telemetry.record_connect();

    let socket_id = Uuid::new_v4();
    let (tx, mut out_rx) = mpsc::channel::<Bytes>(SOCKET_BUFFER_CAPACITY);
    let socket = SocketTx { socket_id, tx };

    let mut write_session = session.clone();
    actix_web::rt::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if write_session.binary(bytes).await.is_err() {
                // The transport is broken. Close the session from this side
                // so the reader's MessageStream wakes up immediately and
                // calls on_disconnect — otherwise every subsequent dispatch
                // sits in `Closed` for up to CLIENT_TIMEOUT (10s) before the
                // reader's heartbeat times out.
                let _ = write_session.close(None).await;
                break;
            }
        }
    });

    let hub = hub.get_ref().clone();
    let data = Arc::clone(&data);
    let pool = pool.get_ref().clone();
    let jwt_secret = jwt_secret.into_inner();
    actix_web::rt::spawn(reader_task(
        session,
        msg_stream,
        socket,
        hub,
        data,
        pool,
        jwt_secret,
        user_uid,
        username,
        admin,
        authed,
    ));

    Ok(response)
}

async fn resolve_identity(
    identity: Option<Identity>,
    pool: &DbPool,
) -> (Uuid, String, bool, bool) {
    let anonymous = || {
        let id = Uuid::new_v4();
        (id, id.to_string(), false, false)
    };

    // Identity cookie (SSR + hydrate same-origin path). Cross-origin
    // clients (HiveGame mobile) start anonymous and upgrade via a
    // `ClientRequest::Auth(token)` frame sent immediately after open.
    let Some(id) = identity else {
        log::debug!("WS connect (anonymous): no identity cookie");
        return anonymous();
    };

    let Ok(id_string) = id.id() else {
        log::warn!("WS connect: identity cookie present but id() failed");
        return anonymous();
    };

    let Ok(uuid) = Uuid::parse_str(&id_string) else {
        log::warn!("WS connect: identity id is not a valid UUID");
        return anonymous();
    };

    load_user(uuid, pool, "cookie").await.unwrap_or_else(anonymous)
}

async fn load_user(uuid: Uuid, pool: &DbPool, source: &str) -> Option<(Uuid, String, bool, bool)> {
    match get_conn(pool).await {
        Ok(mut conn) => match User::find_by_uuid(&uuid, &mut conn).await {
            Ok(user) => {
                log::debug!("WS connect ({source}): user {} authed", user.username);
                Some((uuid, user.username, user.admin, true))
            }
            Err(_) => None,
        },
        Err(err) => {
            log::warn!("WS connect ({source}): DB pool unavailable: {err}");
            None
        }
    }
}
