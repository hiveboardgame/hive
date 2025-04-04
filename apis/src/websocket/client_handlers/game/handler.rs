use super::{
    reaction::handler::handle_reaction, tv::handler::handle_tv, urgent::handler::handle_urgent,
};
use crate::{common::GameUpdate, providers::UpdateNotifier};
use leptos::{logging, prelude::*};

pub fn handle_game(game_update: GameUpdate) {
    //logging::log!("handle_game");
    let game_updater = expect_context::<UpdateNotifier>();
    match game_update {
        GameUpdate::Reaction(game) => handle_reaction(game),
        GameUpdate::Tv(game) => handle_tv(game),
        GameUpdate::Urgent(games) => handle_urgent(games),
        GameUpdate::Heartbeat(hb) => {
            logging::log!("Got heartbeat: {hb:?}");
            game_updater.heartbeat.set(hb);
        }
    }
}
