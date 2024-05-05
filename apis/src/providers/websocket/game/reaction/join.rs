use crate::{
    common::server_result::GameActionResponse,
    providers::{
        game_state::GameStateSignal, games::GamesSignal, timer::TimerSignal,
        websocket::game::reaction::handler::reset_game_state,
    },
};
use hive_lib::game_control::GameControl;
use leptos::*;

pub fn handle_join(gar: GameActionResponse) {
    let _games = expect_context::<GamesSignal>();
    let game_state = expect_context::<GameStateSignal>();
    game_state.loaded.set(false);
    //log!("joined the game, reconstructing game state");
    reset_game_state(&gar.game);
    let timer = expect_context::<TimerSignal>();
    timer.update_from(&gar.game);
    //game_state has loaded
    game_state.loaded.set(true);
    // TODO: @leex try this again once the play page works correctly.
    if let Some((_turn, gc)) = gar.game.game_control_history.last() {
        //log!("Got a GC: {}", gc);
        match gc {
            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                game_state.set_pending_gc(gc.clone())
            }
            _ => {}
        }
    }
    // TODO: @leex
    // Check here whether it's one of your own GCs and only show it when it's not
    // your own GC also only if user is a player.
}
