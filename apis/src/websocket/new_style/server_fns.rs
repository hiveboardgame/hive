use crate::functions::auth::identity::uuid;
use crate::{
    common::ServerMessage,
    websocket::{lag_tracking::PingStats, InternalServerMessage, MessageDestination},
};
use futures::{channel::mpsc::Sender, sink::SinkExt};
use rand::Rng;
use server_fn::ServerFnError;
use std::{ops::DerefMut, sync::Arc};
use tokio::sync::{watch::Receiver, RwLock};
use tokio::time::{interval as interval_fn, Duration};

pub async fn ping_client_every_ms(
    interval: u64,
    mut client: Sender<Result<ServerMessage, ServerFnError>>,
    server_data: Arc<RwLock<PingStats>>,
) {
    let mut interval = interval_fn(Duration::from_millis(interval));
    loop {
        interval.tick().await;
        let nonce = rand::rng().random();
        let mut pings = server_data.write().await;
        pings.deref_mut().set_nonce(nonce);
        let message = ServerMessage::Ping {
            nonce,
            value: pings.deref_mut().value(),
        };
        let res = client.send(Ok(message)).await;
        if res.is_err() {
            println!("Client disconected");
            break;
        }
    }
}

pub async fn handle_server_notificantions(
    mut client: Sender<Result<ServerMessage, ServerFnError>>,
    mut server_reciever: Receiver<InternalServerMessage>,
) {
    loop {
        if server_reciever.changed().await.is_ok() {
            let InternalServerMessage {
                destination,
                message,
            } = server_reciever.borrow().clone();
            let response = match destination {
                MessageDestination::Global => Some(ServerMessage::Error(format!(
                    "Got global notification: {message:?}"
                ))),
                MessageDestination::User(id) => {
                    if Ok(id) == uuid().await {
                        Some(ServerMessage::Error(format!(
                            "Got user notification: {message:?}"
                        )))
                    } else {
                        None
                    }
                }
                _ => {
                    todo!()
                }
            };
            if let Some(response) = response {
                let res = client.send(Ok(response)).await;
                if res.is_err() {
                    println!("Client {} disconnected", uuid().await.unwrap_or_default());
                }
            }
        }
    }
}
