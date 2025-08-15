use crate::{
    common::{ClientRequest, ServerMessage},
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
    use crate::websocket::{
        new_style::{server_fns, ServerNotifications},
        InternalServerMessage, MessageDestination,
    };
    use actix_web::web::Data;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let pings = Arc::new(RwLock::new(PingStats::default()));
    let mut input = input; // FIXME :-) server fn fields should pass mut through to destructure

    let server_notifications = req
        .app_data::<Data<ServerNotifications>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .get_ref();
    let server_sender = server_notifications.sender();

    // create a channel of outgoing websocket messages (from mpsc)
    let (mut client_sender, rx) = mpsc::channel(1);

    //ping at a given interval
    const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation
    leptos::task::spawn(server_fns::ping_client_every_ms(
        PING_INTERVAL_MS,
        client_sender.clone(),
        pings.clone(),
    ));

    //listens to the server notifications and sends them to the client
    leptos::task::spawn(server_fns::handle_server_notificantions(
        client_sender.clone(),
        server_notifications.receiver(),
    ));
    //recieves client requests and handles them
    leptos::task::spawn(async move {
        while let Some(msg) = input.next().await {
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
                    _ => ServerMessage::Error("Not implemented".to_string()),
                },
                Err(e) => ServerMessage::Error(format!("Error: {e}")),
            };
            let res = client_sender.send(Ok(msg)).await;
            if res.is_err() {
                println!("Client {} disconnected", uuid().await.unwrap_or_default());
            }
        }
    });

    Ok(rx.into())
}
