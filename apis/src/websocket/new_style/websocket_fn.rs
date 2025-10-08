use crate::common::{ClientRequest, ServerMessage};
use futures::{channel::mpsc, StreamExt};
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::websocket::{
        new_style::server::{jobs, ClientData, ServerData},
     };
    use actix_web::web::Data;

    let mut input = input;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server_data = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone();
    let id = uuid().await.ok();
    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);
    let client_data = ClientData::new(tx, id);

    //Load initial online users and add myself
    leptos::task::spawn(
        jobs::load_online_users(client_data.clone(), server_data.clone())
    );
    //ping at a given interval
    const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation
    leptos::task::spawn(jobs::ping_client_ms(
        PING_INTERVAL_MS,
        client_data.clone(),
        server_data.clone(),
    ));

    //listens to the server notifications and sends them to the client
    leptos::task::spawn(jobs::handle_server_notificantions(
        client_data.clone(),
        server_data.clone(),
    ));
    //recieves client requests and handles them
    leptos::task::spawn(async move {
        let mut client_data = client_data.clone();
        while let Some(msg) = input.next().await {
            match msg {
                Ok(msg) => match msg {
                    ClientRequest::Pong(nonce) => {
                        client_data.update_pings(nonce).await;
                    }
                    ClientRequest::Disconnect => {
                        let id = client_data.id;
                        leptos::logging::log!("Got disconection request from {id:?}");
                        client_data.close(&server_data).await;
                    }
                    c => {
                        let msg = ServerMessage::Error(format!("{c:?} ISNT IMPLEMENTED"));
                        client_data.send(msg, &server_data).await;
                    }
                },
                Err(e) => {
                    let msg = ServerMessage::Error(format!("Error: {e}"));
                    client_data.send(msg, &server_data).await;
                }
            };
            if client_data.is_closed().await {
                break;
            }
        }
    });

    Ok(rx.into())
}
