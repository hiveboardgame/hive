use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::websocket::new_style::server::{tasks, ServerData, TabData};
use crate::websocket::{InternalServerMessage, MessageDestination};

pub async fn server_notifications(client: &TabData, server: &ServerData, mut stream: BroadcastStream<InternalServerMessage>) {
    //Load initial online users and add myself
    while let Some(Ok(InternalServerMessage {
        destination,
        message,
    })) = stream.next().await
    {
        match destination {
            MessageDestination::Global => {
                client.send(message, server);
            }
            MessageDestination::User(dest_id) => {
                if client.account().is_some_and(|u| u.user.uid == dest_id) {
                    client.send(message, server);
                }
            }
            MessageDestination::Game(game_id) => {
                let is_subscriber = server.is_game_subscriber(client, &game_id);
                if is_subscriber {
                    client.send(message.clone(), server);
                }
            }
            MessageDestination::Tournament(tournament_id) => {
                let is_subscriber = server.is_tournament_subscriber(client, &tournament_id);
                if is_subscriber {
                    client.send(message.clone(), server);
                }
            }
            _ => {
                todo!()
            }
        }
    }
}
