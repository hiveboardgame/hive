use crate::{
    common::{ClientRequest, ServerMessage},
    functions::accounts::get::get_account,
};
use futures::channel::mpsc;
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use crate::functions::db::pool;
    use crate::websocket::new_style::server::{
        server_handler,
        tasks::{self, spawn_abortable},
        ClientData, ServerData,
    };
    use actix_web::web::Data;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone();
    let user = get_account().await.ok();
    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);

    // Store the handle so we can stop it later
    let client = ClientData::new(tx, user, pool().await?);
    //ping at a given interval
    const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation
    let ping = tasks::ping_client_ms(PING_INTERVAL_MS, client.clone(), server.clone());
    spawn_abortable(ping, client.token());

    //listens to the server notifications and sends them to the client
    let server_notifications = tasks::subscribe_to_notifications(client.clone(), server.clone());
    spawn_abortable(server_notifications, client.token());

    //Load initial online users and add myself
    let load_users = tasks::load_online_users(client.clone(), server.clone());
    spawn_abortable(load_users, client.token());

    //main handler
    let main_handler = server_handler(input, client.clone(), server);
    spawn_abortable(main_handler, client.token());
    Ok(rx.into())
}
