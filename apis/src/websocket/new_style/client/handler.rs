use crate::{
    common::{ClientRequest, ServerMessage},
    providers::PingContext,
    websocket::{
        client_handlers::{game::handle_game, user_status::handle::handle_user_status},
        new_style::{client::{api::ClientResult, ClientApi}, websocket_fn::websocket_fn},
    },
};
use futures::channel::mpsc::{Receiver};
use futures::StreamExt;
use leptos::prelude::expect_context;

pub async fn client_handler(rx: Receiver<ClientResult>) {
    let mut ping = expect_context::<PingContext>();
    let client_api = expect_context::<ClientApi>();
    match websocket_fn(rx.into()).await {
        Ok(mut messages) => {
            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => match msg {
                        ServerMessage::Ping { nonce, value } => {
                            ping.update_ping(value);
                            client_api.send(ClientRequest::Pong(nonce)).await;
                        }
                        ServerMessage::UserStatus(user_update) => {
                            handle_user_status(user_update);
                        }
                        ServerMessage::Game(game_update) => {
                            handle_game(*game_update);
                        }
                        _ => todo!(),
                    },
                    Err(e) => {
                        leptos::logging::log!("{e}");
                    }
                }
            }
        }
        Err(e) => leptos::logging::warn!("{e}"),
    }
}
