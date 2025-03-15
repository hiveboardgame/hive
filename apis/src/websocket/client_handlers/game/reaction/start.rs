use super::handler::reset_game_state;
use crate::{
    common::GameActionResponse,
    providers::{
        navigation_controller::NavigationControllerSignal, timer::TimerSignal,
        tournaments::TournamentStateContext,
    },
};
use leptos::prelude::*;

pub fn handle_start(gar: GameActionResponse) {
    let navi = expect_context::<NavigationControllerSignal>();
    if navi.game_signal.get().game_id != Some(gar.game_id.clone()) {
        return;
    }
    let ready = expect_context::<TournamentStateContext>().ready;
    reset_game_state(&gar.game);
    let timer = expect_context::<TimerSignal>();
    timer.update_from(&gar.game);
    ready.update(|r| {
        r.remove(&gar.game_id);
    });
}
