use crate::{
    common::GameActionResponse,
    providers::{
        navigation_controller::NavigationControllerSignal, timer::TimerSignal,
        tournaments::TournamentStateContext,
    },
};
use leptos::prelude::*;

pub fn handle_start(gar: GameActionResponse) {
    let ready = expect_context::<TournamentStateContext>().ready;
    // TODO: fix tournament start
    //reset_game_state(&gar.game);
    let timer = expect_context::<TimerSignal>();
    timer.update_from(&gar.game);
    ready.update(|r| {
        r.remove(&gar.game_id);
    });
}
