use actix_web::web::Data;
use futures::StreamExt;
use server_fn::{BoxedStream, ServerFnError};
use uuid::Uuid;

use crate::{
    common::{ClientRequest, GameAction, ServerMessage},
    websocket::{
        new_style::server::{ClientData, ServerData},
        server_handlers::game::handler::GameActionHandler,
    },
};

pub async fn server_handler(
    mut input: BoxedStream<ClientRequest, ServerFnError>,
    client: ClientData,
    server: Data<ServerData>,
) {
    while let Some(msg) = input.next().await {
        match msg {
            Ok(msg) => match msg {
                ClientRequest::Pong(nonce) => {
                    client.update_pings(nonce);
                }
                ClientRequest::Game { game_id, action } => match action {
                    GameAction::Join => {
                        server.subscribe_client_to(&client, game_id.clone());
                        println!("Subscribed {:?} to {}", client.uuid(), game_id);
                    }
                    GameAction::Turn(turn) => {
                        let user_details = if let Some(user) = client.account() {
                            (user.username.as_str(), user.id)
                        } else {
                            ("", Uuid::default())
                        };
                        if let Ok(handler) = GameActionHandler::new(
                            &game_id,
                            GameAction::Turn(turn),
                            user_details,
                            client.pool(),
                        )
                        .await
                        {
                            let msg = handler.handle().await;
                            if let Ok(msg) = msg {
                                for m in msg {
                                    server.send(m).expect("Send internal server message");
                                }
                            }
                        }
                    }
                    _ => leptos::logging::log!("Need to handle {action}"),
                },
                c => {
                    let msg = ServerMessage::Error(format!("{c:?} ISNT IMPLEMENTED"));
                    client.send(msg, &server).await;
                }
            },
            Err(e) => {
                let msg = ServerMessage::Error(format!("Error: {e}"));
                client.send(msg, &server).await;
            }
        };
    }
}
