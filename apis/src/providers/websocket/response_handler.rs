use crate::{
    common::server_result::{ServerMessage, ServerResult},
    providers::{game_state::GameStateSignal, games::GamesSignal},
};

use leptos::logging::log;
use leptos::*;

use super::{
    challenge::handler::handle_challenge, game::handler::handle_game, ping::handle::handle_ping,
    user_status::handle::handle_user_status,
};

pub fn handle_response(m: String) {
    // TODO: @leex this needs to be broken up this is getting out of hand
    let _game_state = expect_context::<GameStateSignal>();
    let _games = expect_context::<GamesSignal>();
    match serde_json::from_str::<ServerResult>(&m) {
        Ok(ServerResult::Ok(ServerMessage::Pong { ping_sent, .. })) => {
            handle_ping(ping_sent);
        }
        Ok(ServerResult::Ok(ServerMessage::UserStatus(user_update))) => {
            handle_user_status(user_update)
        }
        Ok(ServerResult::Ok(ServerMessage::Game(game_update))) => {
            handle_game(game_update);
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(challenge))) => {
            handle_challenge(challenge);
        }
        Ok(ServerResult::Err(e)) => log!("Got error from server: {e}"),
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
        todo => {
            log!("Got {todo:?} which is currently still unimplemented");
        } // GameRequiresAction, UserStatusChange, ...
    }
}
