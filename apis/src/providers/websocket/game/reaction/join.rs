use crate::{
    common::GameActionResponse,
    providers::{
        game_state::GameStateSignal, timer::TimerSignal,
        websocket::game::reaction::handler::reset_game_state,
    },
};
use hive_lib::GameControl;
use leptos::*;

pub fn handle_join(gar: GameActionResponse) {
    let game_state = expect_context::<GameStateSignal>();
    game_state.loaded.set(false);
    reset_game_state(&gar.game);
    let timer = expect_context::<TimerSignal>();
    timer.update_from(&gar.game);
    game_state.loaded.set(true);
    if let Some((_turn, gc)) = gar.game.game_control_history.last() {
        match gc {
            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                game_state.set_pending_gc(gc.clone())
            }
            _ => {}
        }
    }
}
