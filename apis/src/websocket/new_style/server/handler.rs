use std::{sync::Arc, vec};

use futures::StreamExt;
use server_fn::{BoxedStream, ServerFnError};
use crate::{
    common::{ClientRequest, GameAction, ServerMessage},
    websocket::{
        new_style::server::{ServerData, TabData},
        server_handlers::{challenges::handler::ChallengeHandler, game::handler::GameActionHandler, schedules::ScheduleHandler},
    },
};

pub async fn server_handler(
    mut input: BoxedStream<ClientRequest, ServerFnError>,
    client: TabData,
    server: Arc<ServerData>,
) {
    while let Some(msg) = input.next().await {
        let messages =  async{ match msg {
            Ok(msg) => match msg {
                ClientRequest::Pong(nonce) => {
                    client.update_pings(nonce);
                    Ok(vec![])
                }
                ClientRequest::Game { game_id, action } => {
                    if matches!(action, GameAction::Join) {
                        server.subscribe_client_to(&client, game_id.clone());
                        Ok(vec![])
                    }
                    else {
                        GameActionHandler::new(
                            &game_id,
                            action,
                            client.clone(),
                        )
                        .await?.handle().await
                    }
                },
                ClientRequest::Challenge(c) => {
                    if client.account().is_some() {
                        ChallengeHandler::new(c, client.clone())
                        .await?
                        .handle()
                        .await
                    } else {
                        println!("Anonymous users cant use challenges");
                        Ok(vec![])
                    }
                }
                ClientRequest::Schedule(action) => {
                    if !matches!(action, crate::common::ScheduleAction::TournamentPublic(_)) && client.account().is_none() {
                        let err = "Unauthorized user updated schedules";
                        let msg = ServerMessage::Error(err.to_string());
                        println!("{err}");
                        client.send(msg, &server).await;
                        Ok(vec![])
                    } else {
                        ScheduleHandler::new(action, client.clone())
                        .await?
                        .handle()
                        .await
                    }

                }
                c => {
                    let msg = ServerMessage::Error(format!("{c:?} ISNT IMPLEMENTED"));
                    client.send(msg, &server).await;
                    Ok(vec![])
                }
            },
            Err(e) => {
                let msg = ServerMessage::Error(format!("Error: {e}"));
                client.send(msg, &server).await;
                Ok(vec![])
            }
        }};
        if let Ok(messages) = messages.await{
            for m in messages {
                server.send(m).expect("Send internal server message");
            }
        } 
    }
}
