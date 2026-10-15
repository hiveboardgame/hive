use std::sync::Arc;

use crate::{
    api::v1::auth::jwt_secret::JwtSecret,
    common::WEBSOCKET_PROTOCOL_VERSION,
    security::origin::validate_request_origin,
    websocket::{
        messages::{SocketFormat, SocketTx},
        ws_connection::{reader_task, Deps},
        ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY},
        WebsocketData,
    },
};
use actix_identity::Identity;
use actix_web::{
    get,
    http::header::ORIGIN,
    web::{Data, Payload, Query},
    Error,
    HttpRequest,
    HttpResponse,
};
use bytes::Bytes;
use db_lib::{get_conn, models::User, DbPool};
use serde::Deserialize;
use shared_types::SimpleUser;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Deserialize)]
struct SocketProtocol {
    v: Option<u16>,
    format: Option<String>,
}

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
    // Only a cookie-carrying handshake can be hijacked cross-site. Bots connect without a
    // cookie and without the Origin header browsers always send, then authenticate in-band.
    if let Err(error) = validate_request_origin(&req, identity.is_some()) {
        data.telemetry.record_handshake_fail();
        return Err(error);
    }
    let protocol = Query::<SocketProtocol>::from_query(req.query_string())?;
    let format = if protocol.format.as_deref() == Some("json") {
        SocketFormat::Json
    } else {
        SocketFormat::Msgpack
    };
    // Bots use the public API without the browser's protocol-version parameter.
    // Browser WebSocket handshakes always carry Origin, including anonymous tabs.
    let browser_handshake = identity.is_some() || req.headers().contains_key(ORIGIN);
    if browser_handshake
        && format == SocketFormat::Msgpack
        && protocol.v != Some(WEBSOCKET_PROTOCOL_VERSION)
    {
        data.telemetry.record_handshake_fail();
        return Ok(HttpResponse::UpgradeRequired().finish());
    }
    // Until the previous instance has stopped, its sockets may still be raising events this
    // hub never sees; clients retry and connect once the handoff is complete.
    if !crate::active_instance::is_active_instance() {
        return Ok(HttpResponse::ServiceUnavailable().finish());
    }
    let user = resolve_identity(identity, &pool).await;

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
    let socket = SocketTx {
        socket_id,
        format,
        tx,
    };

    let mut write_session = session.clone();
    actix_web::rt::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            // The hub already encoded for this socket's format; the writer only picks the frame
            // type that carries it.
            let written = match format {
                SocketFormat::Msgpack => write_session.binary(bytes).await,
                SocketFormat::Json => {
                    write_session
                        .text(String::from_utf8_lossy(&bytes).into_owned())
                        .await
                }
            };
            if written.is_err() {
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
        Deps {
            hub,
            data,
            pool,
            jwt_secret,
        },
        user,
    ));

    Ok(response)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::origin::ApplicationOrigin;
    use actix_identity::IdentityMiddleware;
    use actix_session::{storage::CookieSessionStore, SessionMiddleware};
    use actix_web::{cookie::Key, http::StatusCode, test, App};
    use db_lib::get_pool;
    use std::sync::atomic::Ordering;

    #[actix_web::test]
    async fn protocol_gate_rejects_stale_tabs_and_accepts_unversioned_bots() {
        let pool = get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("lazy pool");
        let data = Arc::new(WebsocketData::default());
        let hub = WsHub::new(Arc::clone(&data), pool.clone());
        let app = test::init_service(
            App::new()
                .wrap(IdentityMiddleware::default())
                .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    Key::from(&[0; 64]),
                ))
                .app_data(Data::new(pool))
                .app_data(Data::new(JwtSecret::new("test-secret".to_string())))
                .app_data(Data::new(hub.clone()))
                .app_data(Data::from(Arc::clone(&data)))
                .app_data(Data::new(
                    ApplicationOrigin::parse("https://hivegame.com").unwrap(),
                ))
                .service(start_connection),
        )
        .await;

        for uri in [
            "/ws/".to_string(),
            format!("/ws/?v={}", WEBSOCKET_PROTOCOL_VERSION + 1),
        ] {
            let request = test::TestRequest::get()
                .uri(&uri)
                .insert_header((ORIGIN, "https://hivegame.com"))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        }
        assert!(hub.sessions.is_empty());
        assert_eq!(data.telemetry.connections_total.load(Ordering::Relaxed), 0);
        assert_eq!(
            data.telemetry
                .load_user_state_queued
                .load(Ordering::Relaxed),
            0
        );
        assert_eq!(
            data.telemetry
                .load_user_state_in_flight
                .load(Ordering::Relaxed),
            0
        );

        for uri in ["/ws/?format=json", "/ws/"] {
            let request = test::TestRequest::get()
                .uri(uri)
                .insert_header(("upgrade", "websocket"))
                .insert_header(("connection", "upgrade"))
                .insert_header(("sec-websocket-version", "13"))
                .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        }
    }
}
