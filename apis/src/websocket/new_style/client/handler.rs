use crate::{
    common::ServerMessage,
    providers::PingContext,
    websocket::client_handlers::{
        challenge::handler::handle_challenge, game::handle_game,
        schedule::handler::handle_schedule, tournament::handler::handle_tournament,
        user_status::handle::handle_user_status,
    },
    websocket::new_style::client::ClientApi,
};
use futures::StreamExt;
use futures::Stream;
use server_fn::ServerFnError;

pub async fn client_handler<S>(
    client_api: ClientApi,
    ping: PingContext,
    mut stream: S,
) where
    S: Stream<Item = Result<ServerMessage, ServerFnError>> + Unpin,
{
    client_api.game_join();
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => match msg {
                ServerMessage::Ping { nonce, value } => {
                    ping.update_ping(value);
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
                leptos::logging::log!("WS restart");
             }
        }
    }
}
