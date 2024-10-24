use super::{
    control::handle_control, join::handle_join, new::handle_new_game, timeout::handle_timeout,
    turn::handle_turn,
};
use crate::{
    common::{GameActionResponse, GameReaction},
    providers::{game_state::GameStateSignal, games::GamesSignal},
    responses::GameResponse,
    websocket::client_handlers::game::tv::handler::handle_tv,
};
use hive_lib::{GameStatus, History, State};
use leptos::*;

use super::{ready::handle_ready, start::handle_start};

pub fn handle_reaction(gar: GameActionResponse) {
    let _games = expect_context::<GamesSignal>();
    let _game_state = expect_context::<GameStateSignal>();
    //log!("Got a game action response message: {:?}", gar);
    match gar.game_action.clone() {
        GameReaction::New => {
            handle_new_game(gar.game.clone());
        }
        GameReaction::Tv => {
            handle_tv(gar.game.clone());
        }
        GameReaction::TimedOut => {
            handle_timeout(gar.clone());
        }
        GameReaction::Turn(ref turn) => {
            handle_turn(turn.clone(), gar.clone());
        }
        GameReaction::Control(ref game_control) => {
            handle_control(game_control.clone(), gar.clone())
        }
        GameReaction::Join => {
            handle_join(gar.clone());
        }
        GameReaction::Started => {
            handle_start(gar.clone());
        }
        GameReaction::Ready => {
            handle_ready(gar.clone());
        }
    };
}

pub fn reset_game_state_for_takeback(game: &GameResponse) {
    let mut game_state = expect_context::<GameStateSignal>();
    game_state.view_game();
    game_state.set_game_response(game.clone());
    let mut history = History::new();
    game.history.clone_into(&mut history.moves);
    game.game_type.clone_into(&mut history.game_type);
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, game.black_player.uid, game.white_player.uid);
    };
}

pub fn reset_game_state(game: &GameResponse) {
    let mut game_state = expect_context::<GameStateSignal>();
    batch(move || {
        game_state.full_reset();
        game_state
            .signal
            .update_untracked(|gs| gs.game_id = Some(game.game_id.clone()));
        game_state.set_game_response(game.clone());
        let mut history = History::new();
        game.history.clone_into(&mut history.moves);
        game.game_type.clone_into(&mut history.game_type);
        if let GameStatus::Finished(result) = &game.game_status {
            result.clone_into(&mut history.result);
        }
        if let Ok(state) = State::new_from_history(&history) {
            game_state.set_state(state, game.black_player.uid, game.white_player.uid);
        }
    });
}
