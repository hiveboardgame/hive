use crate::{
    common::GameUpdate,
    providers::websocket::game::{
        reaction::handler::handle_reaction, tv::handler::handle_tv, urgent::handler::handle_urgent,
    },
};

use super::heartbeat::handler::handle_heartbeat;

pub fn handle_game(game_update: GameUpdate) {
    match game_update {
        GameUpdate::Reaction(game) => handle_reaction(game),
        GameUpdate::Tv(game) => handle_tv(game),
        GameUpdate::Urgent(games) => handle_urgent(games),
        GameUpdate::Heartbeat(heartbeat) => handle_heartbeat(heartbeat),
    }
}
