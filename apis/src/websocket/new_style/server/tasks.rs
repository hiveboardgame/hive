use crate::common::UserUpdate;
use crate::websocket::new_style::server::{ClientData, ServerData};
use crate::{
    common::ServerMessage,
    websocket::{InternalServerMessage, MessageDestination},
};
use actix_web::web::Data;
use rand::Rng;
use tokio::time::{interval as interval_fn, Duration};

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

pub async fn handle_server_notificantions(client: ClientData, server_data: Data<ServerData>) {
    let mut server_reciever = server_data.receiver();
    while server_reciever.changed().await.is_ok() {
            let msg = server_reciever.borrow().clone();
            let InternalServerMessage {
                destination,
                message,
            } = msg;
            match destination {
                MessageDestination::Global =>{
                    client.send(message, &server_data).await;
                },
                MessageDestination::User(dest_id) => {
                    if let Some(user) = client.account()  {
                        if user.user.uid == dest_id {
                            client.send(message, &server_data).await;

                        }
                    }
                }
                MessageDestination::Game(id) => {
                    let subscribers = server_data.game_subscribers(&id);
                    let message = message.clone();
                    for c in subscribers {
                        c.send(message.clone(), &server_data).await;
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
        let request = ServerMessage::UserStatus(
            UserUpdate {
                status: crate::common::UserStatus::Online,
                user
            }
        );
        client.send(request, &server_data).await;
    }
    if let Some(user) = client.account() { 
        server_data.add_user(user.clone());
    }
}
