use std::future::Future;
use std::sync::Arc;

use crate::common::UserUpdate;
use crate::websocket::new_style::server::{TabData, ServerData};
use crate::{
    common::ServerMessage,
    websocket::{InternalServerMessage, MessageDestination},
};
use rand::Rng;
use tokio::{
    time::{interval as interval_fn, Duration},
};
use tokio_stream::StreamExt;
use tokio::{spawn, select};
use tokio_util::sync::CancellationToken;

const PING_INTERVAL_MS: u64 = 1000; //consistent with previous implementation

pub async fn ping_client(client: TabData, server_data: Arc<ServerData>) {
    let mut interval = interval_fn(Duration::from_millis(PING_INTERVAL_MS));
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

pub async fn subscribe_to_notifications(client: TabData, server: Arc<ServerData>) {
    let mut reciever = server.notifications();
    while let Some(Ok(InternalServerMessage {
        destination,
        message,
    })) = reciever.next().await {
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
                let is_subscriber = server.is_game_subscriber(&client, &game_id);
                if is_subscriber {
                    client.send(message.clone(), &server).await;
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

pub async fn load_online_users(client: TabData, server_data: Arc<ServerData>) {
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
