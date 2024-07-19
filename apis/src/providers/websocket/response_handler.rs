use crate::common::{ServerMessage::*, ServerResult};

use leptos::logging::log;
use leptos::*;

use super::{
    challenge::handler::handle_challenge, chat::handle::handle_chat, game::handler::handle_game,
    ping::handle::handle_ping, tournament::handler::handle_tournament,
    user_search::handle::handle_user_search, user_status::handle::handle_user_status,
};

pub fn handle_response(m: String) {
    batch(move || match serde_json::from_str::<ServerResult>(&m) {
        Ok(result) => match result {
            ServerResult::Ok(message) => match *message {
                Ping { value, nonce } => handle_ping(nonce, value),
                UserStatus(user_update) => handle_user_status(user_update),
                Game(game_update) => handle_game(*game_update),
                Challenge(challenge) => handle_challenge(challenge),
                Chat(message) => handle_chat(message),
                UserSearch(results) => handle_user_search(results),
                Tournament(tournament_update) => handle_tournament(tournament_update),
                todo => {
                    log!("Got {todo:?} which is currently still unimplemented");
                }
            },
            ServerResult::Err(e) => log!("Got error from server: {e}"),
        },
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
    });
}
