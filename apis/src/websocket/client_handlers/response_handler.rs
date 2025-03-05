use super::{
    challenge::handler::handle_challenge, chat::handle::handle_chat, game::handler::handle_game,
    ping::handle::handle_ping, schedule::handler::handle_schedule, tournament::handler::handle_tournament,
    user_status::handle::handle_user_status,
};
use crate::common::{ServerMessage::*, ServerResult, WebsocketMessage};
use leptos::logging::log;
use leptos_router::hooks::use_navigate;

pub fn handle_response(m: WebsocketMessage) {
    match m {
        WebsocketMessage::Server(result) => match result {
            ServerResult::Ok(message) => match *message {
                Ping { value, nonce } => handle_ping(nonce, value),
                UserStatus(user_update) => handle_user_status(user_update),
                Game(game_update) => handle_game(*game_update),
                Challenge(challenge) => handle_challenge(challenge),
                Chat(message) => handle_chat(message),
                Tournament(tournament_update) => handle_tournament(tournament_update),
                Schedule(schedule_update) => handle_schedule(schedule_update),
                todo => {
                    log!("Got {todo:?} which is currently still unimplemented");
                }
            },
            ServerResult::Err(e) => {
                if e.status_code == http::StatusCode::UNAUTHORIZED {
                    let navegate = use_navigate();
                    navegate("/login", Default::default());
                }
                log!("Got error from server: {e}");
            },
        },
        WebsocketMessage::Client(request) => {
            log!("Got a client request: {request:?}")
        }
    };
}
