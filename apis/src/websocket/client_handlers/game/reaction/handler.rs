use super::{control::handle_control, new::handle_new_game, timeout::handle_timeout};
use crate::{
    common::{GameActionResponse, GameReaction},
    providers::{game_state::GameStateSignal, games::GamesSignal, UpdateNotifier},
    responses::GameResponse,
    websocket::client_handlers::game::tv::handler::handle_tv,
};
use hive_lib::{GameStatus, History, State};
use leptos::prelude::*;

pub fn handle_reaction(gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let update_notifier = expect_context::<UpdateNotifier>();
    //logging::log!("Got a game action response message: {:?}", gar);
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
        GameReaction::Turn(_) => {
            games.own_games_add(gar.game.clone());
            update_notifier.game_response.set(Some(gar.clone()));
        }
        GameReaction::Control(ref game_control) => {
            handle_control(game_control.clone(), gar.clone())
        }
        GameReaction::Started => {
            update_notifier.game_response.set(Some(gar.clone()));
        }
        GameReaction::Ready => {
            update_notifier
                .tournament_ready
                .set((gar.game_id, gar.user_id));
        }
    };
}

pub fn reset_game_state_for_takeback(game: &GameResponse, game_state: &mut GameStateSignal) {
    game_state.view_game();
    game_state.set_game_response(game.clone());
    let mut history = History::new();
    game.history.clone_into(&mut history.moves);
    game.game_type.clone_into(&mut history.game_type);
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, game.black_player.uid, game.white_player.uid);
    };
}

pub fn reset_game_state(game: &GameResponse, mut game_state: GameStateSignal) {
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
}
