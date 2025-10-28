use crate::{
    common::ServerMessage,
    providers::PingContext,
    websocket::{
        client_handlers::{
            challenge::handler::handle_challenge, game::handle_game,
            schedule::handler::handle_schedule, tournament::handler::handle_tournament,
            user_status::handle::handle_user_status,
        },
        new_style::{
            client::{api::ClientResult, ClientApi},
            websocket_fn::websocket_fn,
        },
    },
};
use futures::channel::mpsc;
use futures::StreamExt;
use leptos::prelude::{expect_context, RwSignal, Set};

pub async fn client_handler(last_ping: RwSignal<Option<f64>>, client_api: ClientApi) {
    let (tx, rx) = mpsc::channel(1);
    let mut ping = expect_context::<PingContext>();
    match websocket_fn(rx.into()).await {
        Ok(mut messages) => {
            client_api.set_sender(Some(tx));
            client_api.game_join();
            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => match msg {
                        ServerMessage::Ping { nonce, value } => {
                            ping.update_ping(value);
                            let curr = chrono::offset::Utc::now().timestamp_millis();
                            last_ping.set(Some(curr as f64));
                            client_api.pong(nonce).await;
                        }
                        ServerMessage::UserStatus(user_update) => {
                            handle_user_status(user_update);
                        }
                        ServerMessage::Game(game_update) => {
                            handle_game(*game_update);
                        }
                        ServerMessage::Challenge(update) => {
                            handle_challenge(update);
                        }
                        ServerMessage::Schedule(update) => {
                            handle_schedule(update);
                        }
                        ServerMessage::Tournament(update) => {
                            handle_tournament(update);
                        }
                        ServerMessage::Error(e) => {
                            leptos::logging::log!("ServerMessage::Error {e}");
                        }
                        _ => todo!(),
                    },

                    Err(e) => {
                        leptos::logging::log!("WS restart due to {e}"); 
                        client_api.restart_ws();
                    }
                }
            }
        }
        Err(e) => leptos::logging::warn!("{e}"),
    }
}
