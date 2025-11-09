use crate::{
    common::ServerMessage,
    providers::{ClientApi, PingContext},
    websocket::client_handlers::{
        challenge::handler::handle_challenge, chat::handle::handle_chat, game::handle_game, oauth::handle::handle_oauth, schedule::handler::handle_schedule, tournament::handler::handle_tournament, user_status::handle::handle_user_status
    },
};
use futures::StreamExt;
use futures::Stream;
use leptos_router::hooks::use_navigate;
use server_fn::ServerFnError;

pub async fn client_handler<S>(
    client_api: ClientApi,
    ping: PingContext,
    mut stream: S,
) where
    S: Stream<Item = Result<ServerMessage, ServerFnError>> + Unpin,
{
    client_api.set_ws_ready();
    client_api.game_join();
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => match msg {
                ServerMessage::Ping { nonce, value } => {
                    ping.update_ping(value);
                    client_api.pong(nonce);
                }
                ServerMessage::UserStatus(user_update) => handle_user_status(user_update),
                ServerMessage::Game(game_update) => handle_game(*game_update),
                ServerMessage::Challenge(update) => handle_challenge(update),
                ServerMessage::Schedule(update) => handle_schedule(update),
                ServerMessage::Tournament(update) => handle_tournament(update),
                ServerMessage::RedirectLink(link) => handle_oauth(link),
                ServerMessage::Chat(containers) => handle_chat(containers),
                ServerMessage::Error(e) => {
                    leptos::logging::log!("ServerMessage::Error {e}");
                }
            },
            Err(e) => {
                if let ServerFnError::ServerError(e) = e {
                    leptos::logging::log!("Got ServerError: {e}");
                    let navegate = use_navigate();
                    navegate("/login", Default::default());
                } else {
                    leptos::logging::log!("Got Error: {e}");
                }
             }
        }
    }
}
