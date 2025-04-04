use super::heartbeat::handler::handle_heartbeat;
use super::{
    reaction::handler::handle_reaction, tv::handler::handle_tv, urgent::handler::handle_urgent,
};
use crate::common::GameUpdate;

pub fn handle_game(game_update: GameUpdate) {
    //logging::log!("handle_game");
    match game_update {
        GameUpdate::Reaction(game) => handle_reaction(game),
        GameUpdate::Tv(game) => handle_tv(game),
        GameUpdate::Urgent(games) => handle_urgent(games),
        GameUpdate::Heartbeat(heartbeat) => handle_heartbeat(heartbeat),
    }
}
