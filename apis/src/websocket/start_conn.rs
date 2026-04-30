use std::sync::Arc;

use crate::websocket::{
    messages::SocketTx,
    ws_connection::reader_task,
    ws_hub::WsHub,
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
) -> Result<HttpResponse, Error> {
    let (user_uid, username, admin, authed) = resolve_identity(identity, &pool).await;

    let ws_result = actix_ws::handle(&req, body);
    if ws_result.is_err() {
        data.telemetry.record_handshake_fail();
    }
    let (response, session, msg_stream) = ws_result?;
    data.telemetry.record_connect();

    let socket_id = Uuid::new_v4();
    let (tx, mut out_rx) = mpsc::channel::<Bytes>(128);
    let socket = SocketTx { socket_id, tx };

    let mut write_session = session.clone();
    actix_web::rt::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if write_session.binary(bytes).await.is_err() {
                break;
            }
        }
    });

    let hub = hub.get_ref().clone();
    let data = Arc::clone(&data);
    let pool = pool.get_ref().clone();
    actix_web::rt::spawn(reader_task(
        session, msg_stream, socket, hub, data, pool, user_uid, username, admin, authed,
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

    let Some(id) = identity else {
        println!("Welcome Anonymous!");
        return anonymous();
    };

    let Ok(id_string) = id.id() else {
        println!("Wrong id");
        return anonymous();
    };

    let Ok(uuid) = Uuid::parse_str(&id_string) else {
        println!("Can't parse to Uuid");
        return anonymous();
    };

    match get_conn(pool).await {
        Ok(mut conn) => match User::find_by_uuid(&uuid, &mut conn).await {
            Ok(user) => {
                println!("Welcome {}!", user.username);
                (uuid, user.username, user.admin, true)
            }
            Err(_) => anonymous(),
        },
        Err(err) => {
            println!("Could not establish database connection: {err}");
            anonymous()
        }
    }
}
