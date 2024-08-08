use super::{
    challenge::handler::handle_challenge, chat::handle::handle_chat, game::handler::handle_game,
    games_search::handle::handle_games_search, ping::handle::handle_ping,
    player_profile::handle::handle_player_profile, schedule::handler::handle_schedule,
    tournament::handler::handle_tournament, user_search::handle::handle_user_search,
    user_status::handle::handle_user_status,
};
use crate::common::{CommonMessage, ServerMessage::*, ServerResult};
use leptos::logging::log;
use leptos::*;

pub fn handle_response(m: &CommonMessage) {
    batch(move || match m {
        CommonMessage::Server(result) => match result {
            ServerResult::Ok(message) => match *message.clone() {
                Ping { value, nonce } => handle_ping(nonce, value),
                UserStatus(user_update) => handle_user_status(user_update),
                Game(game_update) => handle_game(*game_update),
                Challenge(challenge) => handle_challenge(challenge),
                Chat(message) => handle_chat(message),
                UserSearch(results) => handle_user_search(results),
                Tournament(tournament_update) => handle_tournament(tournament_update),
                GamesSearch(results) => handle_games_search(results),
                PlayerProfile(results) => handle_player_profile(results),
                Schedule(schedule_update) => handle_schedule(schedule_update),
                todo => {
                    log!("Got {todo:?} which is currently still unimplemented");
                }
            },
            ServerResult::Err(e) => log!("Got error from server: {e}"),
        },
        CommonMessage::Client(request) => {
            log!("Got a client request: {request:?}")
        }
    });
}
