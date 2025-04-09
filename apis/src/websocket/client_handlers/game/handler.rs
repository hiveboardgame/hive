use super::reaction::{handle_control, handle_new_game};
use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate},
    providers::{game_state::GameStateSignal, games::GamesSignal, UpdateNotifier},
    responses::GameResponse,
};
use hive_lib::{GameStatus, History, State};
use leptos::{prelude::*, task::spawn_local};
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use shared_types::GameId;
use uuid::Uuid;

pub fn handle_game(game_update: GameUpdate) {
    let game_updater = expect_context::<UpdateNotifier>();
    let mut games_signal = expect_context::<GamesSignal>();
    match game_update {
        GameUpdate::Reaction(game) => handle_reaction(game),
        GameUpdate::Tv(game) => {
            games_signal.live_games_add(game);
        }
        GameUpdate::Urgent(games) => {
            games_signal.own_games_set(games);
        }
        GameUpdate::Heartbeat(hb) => {
            game_updater.heartbeat.set(hb);
        }
    }
}

fn handle_reaction(gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let update_notifier = expect_context::<UpdateNotifier>();
    match gar.game_action.clone() {
        GameReaction::New => {
            handle_new_game(gar.game.clone());
        }
        GameReaction::Tv => {
            games.live_games_add(gar.game);
        }
        GameReaction::TimedOut => {
            let game_id = &gar.game.game_id;
            games.own_games_remove(game_id);
            games.live_games_remove(game_id);
            update_notifier.game_response.set(Some(gar.clone()));
        }
        GameReaction::Turn(_) => {
            games.own_games_add(gar.game.clone());
            update_notifier.game_response.set(Some(gar.clone()));
        }

        GameReaction::Join => {
            // TODO: Do we want anything here?
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
            spawn_local(async move {
                let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
                    move |_: ()| {
                        update_notifier
                            .tournament_ready
                            .set((GameId::default(), Uuid::default()));
                    },
                    30_000.0,
                );
                start(());
            });
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
