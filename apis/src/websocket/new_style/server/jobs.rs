use crate::common::UserUpdate;
use crate::websocket::new_style::server::{ClientData, ServerData};
use crate::{
    common::ServerMessage,
    websocket::{InternalServerMessage, MessageDestination},
};
use actix_web::web::Data;
use rand::Rng;
use tokio::time::{interval as interval_fn, Duration};

pub async fn ping_client_ms(interval: u64, mut client: ClientData, server_data: Data<ServerData>) {
    let mut interval = interval_fn(Duration::from_millis(interval));
    loop {
        interval.tick().await;
        if client.is_closed().await {
            break;
        }
        let nonce = rand::rng().random();
        client.update_pings(nonce).await;
        let message = ServerMessage::Ping {
            nonce,
            value: client.pings_value().await,
        };
        client.send(message, &server_data).await;
    }
}

pub async fn handle_server_notificantions(mut client: ClientData, server_data: Data<ServerData>) {
    let mut server_reciever = server_data.receiver();
    loop {
        if client.is_closed().await {
            break;
        }
        if server_reciever.changed().await.is_ok() {
            let InternalServerMessage {
                destination,
                message,
            } = server_reciever.borrow().clone();
            match destination {
                MessageDestination::Global =>{
                    client.send(message, &server_data).await;
                },
                MessageDestination::User(dest_id) => {
                    if client.id.is_some_and(|id| id == dest_id) {
                        client.send(message, &server_data).await;
                    }
                }
                _ => {
                    todo!()
                }
            }
        }
    }
}

pub async fn load_online_users(mut client: ClientData, server_data: Data<ServerData>) {
    for user in server_data.get_online_users().await {
        let request = ServerMessage::UserStatus(
            UserUpdate {
                status: crate::common::UserStatus::Online,
                user
            }
        );
        client.send(request, &server_data).await;
    }
    server_data.add_user(client.id).await;
}
