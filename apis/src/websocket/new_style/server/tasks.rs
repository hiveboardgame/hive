use std::future::Future;

use crate::common::UserUpdate;
use crate::websocket::new_style::server::{ClientData, ServerData};
use crate::{
    common::ServerMessage,
    websocket::{InternalServerMessage, MessageDestination},
};
use actix_web::web::Data;
use rand::Rng;
use tokio::{
    select, spawn,
    time::{interval as interval_fn, Duration},
};
use tokio_util::sync::CancellationToken;

pub async fn ping_client_ms(interval: u64, client: ClientData, server_data: Data<ServerData>) {
    let mut interval = interval_fn(Duration::from_millis(interval));
    loop {
        interval.tick().await;
        let nonce = rand::rng().random();
        client.update_pings(nonce);
        let message = ServerMessage::Ping {
            nonce,
            value: client.pings_value(),
        };
        client.send(message, &server_data).await;
    }
}

pub async fn subscribe_to_notifications(client: ClientData, server: Data<ServerData>) {
    let mut reciever = server.receiver();
    while reciever.changed().await.is_ok() {
        let InternalServerMessage {
            destination,
            message,
        } = reciever.borrow().clone();
        match destination {
            MessageDestination::Global => {
                client.send(message, &server).await;
            }
            MessageDestination::User(dest_id) => {
                if client.account().is_some_and(|u| u.user.uid == dest_id) {
                    client.send(message, &server).await;
                }
            }
            MessageDestination::Game(game_id) => {
                let is_subscriber = server.is_game_subscriber(client.uuid(), &game_id);
                let message = message.clone();
                if is_subscriber {
                    client.send(message, &server).await;
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

pub async fn load_online_users(client: ClientData, server_data: Data<ServerData>) {
    println!("Reached load online users");
    for user in server_data.get_online_users() {
        let request = ServerMessage::UserStatus(UserUpdate {
            status: crate::common::UserStatus::Online,
            user,
        });
        client.send(request, &server_data).await;
    }
    if let Some(user) = client.account() {
        server_data.add_user(user.user.clone());
    }
}

pub fn spawn_abortable<F>(task: F, token: CancellationToken)
where
    F: Future<Output = ()> + Send + 'static,
{
    spawn(async move {
        select! {
           _ = token.cancelled() => {}
           _ = task => {}
        }
    });
}
