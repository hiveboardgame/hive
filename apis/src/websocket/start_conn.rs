use std::sync::Arc;

use crate::{
    api::v1::auth::{bearer::resolve_bearer_user, jwt_secret::JwtSecret},
    websocket::{
        messages::SocketTx,
        ws_connection::reader_task,
        ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY},
        WebsocketData,
    },
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
use shared_types::SimpleUser;
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
    let user = match resolve_bearer_identity(&req, &pool, &jwt_secret).await {
        Some(user) => user,
        None => resolve_identity(identity, &pool).await,
    };

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
    actix_web::rt::spawn(reader_task(
        session, msg_stream, socket, hub, data, pool, user,
    ));

    Ok(response)
}

/// Bearer-token auth for native clients that can't hold a browser session
/// cookie. Checked before the cookie-session path; falls through to it
/// (eventually anonymous) on any failure rather than erroring the handshake.
async fn resolve_bearer_identity(
    req: &HttpRequest,
    pool: &DbPool,
    jwt_secret: &JwtSecret,
) -> Option<SimpleUser> {
    let user = resolve_bearer_user(req, pool, jwt_secret).await?;
    log::debug!("WS connect: user {} authed via bearer token", user.username);
    Some(SimpleUser {
        user_id: user.id,
        username: user.username,
        admin: user.admin,
        authed: true,
    })
}

async fn resolve_identity(identity: Option<Identity>, pool: &DbPool) -> SimpleUser {
    let anonymous = || {
        let id = Uuid::new_v4();
        SimpleUser {
            user_id: id,
            username: id.to_string(),
            admin: false,
            authed: false,
        }
    };

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

    match get_conn(pool).await {
        Ok(mut conn) => match User::find_active_by_uuid(&uuid, &mut conn).await {
            Ok(user) => {
                log::debug!("WS connect: user {} authed", user.username);
                SimpleUser {
                    user_id: uuid,
                    username: user.username,
                    admin: user.admin,
                    authed: true,
                }
            }
            Err(_) => anonymous(),
        },
        Err(err) => {
            log::warn!("WS connect: DB pool unavailable, falling back to anonymous: {err}");
            anonymous()
        }
    }
}
