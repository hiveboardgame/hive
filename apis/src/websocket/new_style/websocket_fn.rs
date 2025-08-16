use crate::{
    common::{ClientRequest, ServerMessage},
    functions::accounts::get::get_account,
    websocket::lag_tracking::PingStats,
};
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};
use std::ops::DerefMut;

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use futures::{channel::mpsc, SinkExt, StreamExt};

    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use crate::websocket::{
        new_style::{server_fns, server_types::ClientSender, ServerData},
        InternalServerMessage, MessageDestination,
    };
    use actix_web::web::Data;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let pings = Arc::new(RwLock::new(PingStats::default()));
    let mut input = input; // FIXME :-) server fn fields should pass mut through to destructure

    let server_data = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone();
    let pool = pool().await?;
    let id = uuid().await.ok();
    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);
    let client_sender = ClientSender::new(tx, pool, id);
    //ping at a given interval
    const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation
    leptos::task::spawn(server_fns::ping_client_every_ms(
        PING_INTERVAL_MS,
        client_sender.clone(),
        pings.clone(),
        server_data.clone(),
    ));

    //listens to the server notifications and sends them to the client
    leptos::task::spawn(server_fns::handle_server_notificantions(
        client_sender.clone(),
        server_data.clone(),
    ));
    let server_sender = server_data.sender();
    //recieves client requests and handles them
    leptos::task::spawn(async move {
        let mut client_sender = client_sender.clone();
        while let Some(msg) = input.next().await {
            let id = get_account().await.ok().map(|a| a.id);
            let msg = match msg {
                Ok(msg) => match msg {
                    ClientRequest::DbgMsg(msg) => {
                        if msg.contains("server") {
                            //since we have a cloned sender, we can send global notifications
                            //in respnse to a client request
                            server_sender
                                .send(InternalServerMessage {
                                    destination: MessageDestination::Global,
                                    message: ServerMessage::Error(msg.clone()),
                                })
                                .unwrap();
                            ServerMessage::Error("Sent global notification".to_string())
                        } else {
                            ServerMessage::Error(msg)
                        }
                    }
                    ClientRequest::Pong(nonce) => {
                        pings.write().await.deref_mut().update(nonce);
                        ServerMessage::Error("Pong updated".to_string())
                    }
                    ClientRequest::UpdateId => {
                        let ret = format!("Updated id is: {id:?}");
                        *client_sender.id.write().await = id;
                        leptos::logging::log!("{ret}");
                        ServerMessage::Error(ret)
                    }
                    _ => ServerMessage::Error("Not implemented".to_string()),
                },
                Err(e) => ServerMessage::Error(format!("Error: {e}")),
            };
            let res = client_sender.send(msg, &server_data).await;
            if res.is_err() {
                break;
            }
        }
    });

    Ok(rx.into())
}
