use crate::{
    common::{ClientRequest, ServerMessage},
    providers::PingContext,
    websocket::{
        client_handlers::user_status::handle::handle_user_status,
        new_style::{client::ClientApi, websocket_fn::websocket_fn},
    },
};
use futures::channel::mpsc::{self};
use futures::StreamExt;
use leptos::prelude::expect_context;

pub async fn client_handler() {
    let (tx, rx) = mpsc::channel(1);
    let client_api = expect_context::<ClientApi>();
    let mut ping = expect_context::<PingContext>();
    client_api.set_sender(tx);
    match websocket_fn(rx.into()).await {
        Ok(mut messages) => {
            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => match msg {
                        ServerMessage::Ping { nonce, value } => {
                            ping.update_ping(value);
                            client_api.send(ClientRequest::Pong(nonce));
                        }
                        ServerMessage::UserStatus(user_update) => {
                            handle_user_status(user_update);
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
