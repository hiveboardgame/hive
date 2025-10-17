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
        server_handler,tasks,
        ClientData, ServerData,
    };
    use actix_web::web::Data;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone().into_inner();
    let user = get_account().await.ok();

    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);

    let client = ClientData::new(tx, user, pool().await?);
    //ping at a given interval
    tasks::spawn_abortable(tasks::ping_client(client.clone(),  server.clone()), client.token());

    //listens to the server notifications and sends them to the client
    tasks::spawn_abortable(tasks::subscribe_to_notifications(client.clone(), server.clone()), client.token());

    //Load initial online users and add myself
    tasks::spawn_abortable(tasks::load_online_users(client.clone(), server.clone()), client.token());
    
    //main handler
    tasks::spawn_abortable(server_handler(input,client.clone(), server.clone()), client.token());
    Ok(rx.into())
}
