use crate::{
    common::GameActionResponse,
    providers::{
        game_state::GameStateSignal, games::GamesSignal,
        navigation_controller::NavigationControllerSignal, timer::TimerSignal, SoundType, Sounds,
    },
};
use hive_lib::Turn;
use leptos::prelude::*;

pub fn handle_turn(turn: Turn, gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let sounds = expect_context::<Sounds>();
    games.own_games_add(gar.game.clone());
    let mut game_state = expect_context::<GameStateSignal>();
    let navigation_controller = expect_context::<NavigationControllerSignal>();
    let timer = expect_context::<TimerSignal>();
    if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
        if gar.game.game_id == game_id {
            timer.update_from(&gar.game);
            game_state.clear_gc();
            game_state.set_game_response(gar.game.clone());
            sounds.play_sound(SoundType::Turn);
            if game_state.signal.get_untracked().state.history.moves != gar.game.history {
                match turn {
                    Turn::Move(piece, position) => game_state.play_turn(piece, position),
                    _ => unreachable!(),
                };
            }
        }
    }
}
