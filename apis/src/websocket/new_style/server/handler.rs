use actix_web::web::Data;
use futures::StreamExt;
use server_fn::{BoxedStream, ServerFnError};

use crate::{common::{ClientRequest, GameAction, ServerMessage}, websocket::new_style::server::{ClientData,ServerData}};


pub async fn server_websocket_handler(
    mut input:  BoxedStream<ClientRequest, ServerFnError>, 
    client_data: ClientData,
    server_data: Data<ServerData>
){
    while let Some(msg) = input.next().await {
        match msg {
            Ok(msg) => match msg {
                ClientRequest::Pong(nonce) => {
                    client_data.update_pings(nonce);
                }
                ClientRequest::Game { game_id, action } => {
                    match action {
                        GameAction::Join => {
                            server_data.subscribe_client_to(client_data.clone(), game_id.clone());
                            println!("Subscribed {:?} to {}", client_data.uuid(), game_id);
                            let subs = server_data.game_subscribers(&game_id);
                            let mut x = 0;
                            for sub in subs {
                                x+=1;
                                println!("{x}. {} subed to {game_id}", sub.uuid());
                            }
                        }
                        _ => leptos::logging::log!("Need to handle {action}")
                    }
                }
                c => {
                    let msg = ServerMessage::Error(format!("{c:?} ISNT IMPLEMENTED"));
                    client_data.send(msg, &server_data).await;
                }
            },
            Err(e) => {
                let msg = ServerMessage::Error(format!("Error: {e}"));
                client_data.send(msg, &server_data).await;
            }
        };
    }
}
