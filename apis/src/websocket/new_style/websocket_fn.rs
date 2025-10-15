use crate::{common::{ClientRequest, ServerMessage}, functions::accounts::get::get_account};
use futures::channel::mpsc;
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use crate::websocket::{
        new_style::server::{tasks, ClientData, ServerData, server_websocket_handler},
     };
    use tokio::{task::spawn, select};
    use tokio_util::sync::CancellationToken;
    use actix_web::web::Data;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;


    let server_data = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?.clone();
     let user = get_account().await.ok();
    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);

    // Store the handle so we can stop it later    
    let token = CancellationToken::new();
    let token2 = token.clone();
    let token3 = token.clone();
    let token4 = token.clone();
    let client_data = ClientData::new(tx, user, token.clone()); 
    //ping at a given interval
    const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation
    let ping = tasks::ping_client_ms(
        PING_INTERVAL_MS,
        client_data.clone(),
        server_data.clone(),
    );
    spawn(async move {
        select! {
            _ = token.cancelled() => {}
            _ = ping => {}
         }
    });

    
    //listens to the server notifications and sends them to the client
    let server_notifications = tasks::handle_server_notificantions(
        client_data.clone(),
        server_data.clone(),
    );
    spawn(async move {
        select! {
            _ = token2.cancelled() => {}
            _ = server_notifications => {}
         }
    });
    
    //Load initial online users and add myself
    let load_users = tasks::load_online_users(client_data.clone(), server_data.clone());
    spawn(async move {
        select! {
            _ = token3.cancelled() => {}
            _ = load_users => {}
         }
    });
    //main handler
    let main_handler = server_websocket_handler(input, client_data, server_data);
    spawn(async move {
        select! {
            _ = token4.cancelled() => {}
            _ = main_handler => {}
         }
    });

    Ok(rx.into())
}
