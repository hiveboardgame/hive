use super::handler::reset_game_state;
use crate::{
    common::GameActionResponse,
    providers::{
        game_state::GameStateSignal, navigation_controller::NavigationControllerSignal,
        timer::TimerSignal, tournament_ready::TournamentReadySignal,
    },
};
use leptos::prelude::*;

pub fn handle_start(gar: GameActionResponse) {
    let navi = expect_context::<NavigationControllerSignal>();
    if navi.game_signal.get().game_id != Some(gar.game_id.clone()) {
        return;
    }
    let game_state = expect_context::<GameStateSignal>();
    let ready = expect_context::<TournamentReadySignal>().signal;
    game_state.loaded.set(false);
    reset_game_state(&gar.game);
    let timer = expect_context::<TimerSignal>();
    timer.update_from(&gar.game);
    game_state.loaded.set(true);
    ready.update(|r| {
        r.remove(&gar.game_id);
    });
}
