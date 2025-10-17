use std::{sync::Arc, vec};

use futures::StreamExt;
use server_fn::{BoxedStream, ServerFnError};
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
    server: Arc<ServerData>,
) {
    while let Some(msg) = input.next().await {
        let server_message = match msg {
            Ok(msg) => match msg {
                ClientRequest::Pong(nonce) => {
                    client.update_pings(nonce);
                    vec![]
                }
                ClientRequest::Game { game_id, action } => {
                    if matches!(action, GameAction::Join) {
                        server.subscribe_client_to(&client, game_id.clone());
                        vec![]
                    }
                    else if let Ok(handler) =   GameActionHandler::new(
                        &game_id,
                        action,
                        client.clone(),
                    )
                    .await {
                        handler.handle().await.expect("Handler to return")
                    } else {
                        //errors
                        vec![]
                    }
                },
                c => {
                    let msg = ServerMessage::Error(format!("{c:?} ISNT IMPLEMENTED"));
                    client.send(msg, &server).await;
                    vec![]
                }
            },
            Err(e) => {
                let msg = ServerMessage::Error(format!("Error: {e}"));
                client.send(msg, &server).await;
                vec![]
            }
        };
        for m in server_message {
            server.send(m).expect("Send internal server message");
        }
    }
}
