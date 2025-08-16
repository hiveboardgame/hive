use crate::functions::auth::identity::uuid;
use crate::websocket::new_style::server_types::ClientSender;
use crate::websocket::new_style::ServerData;
use crate::{
    common::ServerMessage,
    websocket::{lag_tracking::PingStats, InternalServerMessage, MessageDestination},
};
use actix_web::web::Data;
use rand::Rng;
use std::{ops::DerefMut, sync::Arc};
use tokio::sync::RwLock;
use tokio::time::{interval as interval_fn, Duration};
pub async fn ping_client_every_ms(
    interval: u64,
    mut client: ClientSender,
    pings: Arc<RwLock<PingStats>>,
    server_data: Data<ServerData>,
) {
    let mut interval = interval_fn(Duration::from_millis(interval));
    let mut x = 0;
    loop {
        interval.tick().await;
        let nonce = rand::rng().random();
        let mut pings = pings.write().await;
        pings.deref_mut().set_nonce(nonce);
        let message = ServerMessage::Ping {
            nonce,
            value: pings.deref_mut().value(),
        };
        let id = *client.id.read().await;
        leptos::logging::log!("{x} before ping send {id:?}");
        let res = client.send(message, &server_data).await;
        leptos::logging::log!("{x} after ping send {id:?}");
        x += 1;
        if let Err(res) = res {
            leptos::logging::log!("{res}");
            break;
        }
    }
}

pub async fn handle_server_notificantions(mut client: ClientSender, server_data: Data<ServerData>) {
    let mut server_reciever = server_data.receiver();
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
                let res = client.send(response, &server_data).await;
                if let Err(res) = res {
                    leptos::logging::log!("{res}");
                    break;
                }
            }
        }
    }
}
