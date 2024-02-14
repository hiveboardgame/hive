use crate::{
    common::{game_reaction::GameReaction, server_result::GameActionResponse},
    providers::{
        game_state::GameStateSignal,
        games::GamesSignal,
        websocket::game::{
            reaction::{
                control::handle_control, join::handle_join, new::handle_new_game,
                timeout::handle_timeout, turn::handle_turn,
            },
            tv::handler::handle_tv,
        },
    },
    responses::game::GameResponse,
};
use hive_lib::{game_status::GameStatus, history::History, state::State};
use leptos::logging::log;
use leptos::*;

pub fn handle_reaction(gar: GameActionResponse) {
    let _games = expect_context::<GamesSignal>();
    let _game_state = expect_context::<GameStateSignal>();
    log!("Got a game action response message: {:?}", gar);
    match gar.game_action.clone() {
        GameReaction::New => {
            handle_new_game(gar.game.clone());
        }
        GameReaction::Tv => {
            handle_tv(gar.game.clone());
        }
        GameReaction::TimedOut => {
            handle_timeout(gar.game_id.clone());
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
    };
}

pub fn reset_game_state(game: &GameResponse) {
    let mut game_state = expect_context::<GameStateSignal>();
    game_state.set_game_response(game.clone());
    let mut history = History::new();
    history.moves = game.history.to_owned();
    history.game_type = game.game_type.to_owned();
    if let GameStatus::Finished(result) = &game.game_status {
        history.result = result.to_owned();
    }
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, game.black_player.uid, game.white_player.uid);
    }
    // TODO: check if there an answered gc and set it
}
