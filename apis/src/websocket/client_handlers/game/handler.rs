use super::reaction::{handle_control, handle_new_game};
use crate::{
    common::{ClientRequest, GameActionResponse, GameReaction, GameUpdate},
    providers::{
        game_state::GameStateSignal,
        games::GamesSignal,
        refocus::RefocusSignal,
        websocket::WebsocketContext,
        AuthContext,
        UpdateNotifier,
    },
    responses::GameResponse,
};
use hive_lib::{GameStatus, History, State};
use leptos::{prelude::*, task::spawn_local};
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use shared_types::{GameId, ReadyUser};
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
        GameUpdate::OwnGameRemoved(game_id) => {
            games_signal.own_games_remove(&game_id);
            games_signal.live_games_remove(&game_id);
        }
        GameUpdate::Heartbeat(hb) => {
            game_updater.heartbeat.set(hb);
        }
    }
}

pub fn handle_tv_snapshot(games: Vec<GameResponse>) {
    let mut games_signal = expect_context::<GamesSignal>();
    games_signal.live_snapshot_apply(games);
}

pub fn handle_urgent_games_snapshot(games: Vec<GameResponse>) {
    let mut games_signal = expect_context::<GamesSignal>();
    games_signal.urgent_snapshot_apply(games);
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
            update_notifier.game_response.set(Some(gar.clone()));
            if gar.game.finished {
                games.own_games_remove(&gar.game.game_id);
                games.live_games_remove(&gar.game.game_id);
            } else {
                games.own_games_add(gar.game.clone());
            }
            ack_seen_if_watching(&gar.game.game_id, gar.game.current_player_id);
        }

        GameReaction::Join => {
            // TODO: Do we want anything here?
        }

        GameReaction::Control(ref game_control) => {
            ack_control_if_watching(*game_control, &gar);
            handle_control(*game_control, gar.clone());
        }
        GameReaction::Started => {
            update_notifier.game_response.set(Some(gar.clone()));
        }
        GameReaction::Ready => {
            let opponent_id = if gar.game.white_player.uid == gar.user_id {
                gar.game.black_player.uid
            } else {
                gar.game.white_player.uid
            };
            update_notifier.tournament_ready.update(|ready_map| {
                ready_map
                    .entry(gar.game_id.clone())
                    .or_default()
                    .push(ReadyUser {
                        proposer_id: gar.user_id,
                        proposer_username: gar.username.clone(),
                        opponent_id,
                    });
                ready_map.retain(|_, users| !users.is_empty());
            });

            let game_id = gar.game_id.clone();
            let user_id = gar.user_id;
            spawn_local(async move {
                let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
                    move |_: ()| {
                        update_notifier.tournament_ready.update(|ready_map| {
                            if let Some(users) = ready_map.get_mut(&game_id) {
                                users.retain(|ready_user| ready_user.proposer_id != user_id);
                                if users.is_empty() {
                                    ready_map.remove(&game_id);
                                }
                            }
                        });
                    },
                    30_000.0,
                );
                start(());
            });
        }
    };
}

fn ack_seen_if_watching(game_id: &GameId, recipient_id: Uuid) {
    let me = expect_context::<AuthContext>()
        .user
        .with_untracked(|a| a.as_ref().map(|account| account.id));
    if me != Some(recipient_id) {
        return;
    }
    send_seen_ack_if_focused(game_id);
}

fn ack_control_if_watching(control: hive_lib::GameControl, gar: &GameActionResponse) {
    let notified = matches!(
        control,
        hive_lib::GameControl::DrawOffer(_)
            | hive_lib::GameControl::TakebackRequest(_)
            | hive_lib::GameControl::DrawReject(_)
            | hive_lib::GameControl::TakebackAccept(_)
            | hive_lib::GameControl::TakebackReject(_)
    );
    if !notified {
        return;
    }
    let Some(me) = expect_context::<AuthContext>()
        .user
        .with_untracked(|a| a.as_ref().map(|account| account.id))
    else {
        return;
    };
    let is_player = me == gar.game.white_player.uid || me == gar.game.black_player.uid;
    if me == gar.user_id || !is_player {
        return;
    }
    send_seen_ack_if_focused(&gar.game.game_id);
}

fn send_seen_ack_if_focused(game_id: &GameId) {
    let focused = expect_context::<RefocusSignal>()
        .signal
        .with_untracked(|s| s.focused);
    let on_this_game = web_sys::window()
        .and_then(|w| w.location().pathname().ok())
        .is_some_and(|p| p == format!("/game/{}", game_id.0));
    if focused && on_this_game {
        expect_context::<WebsocketContext>().send(&ClientRequest::NotificationSeen {
            game_id: game_id.clone(),
        });
    }
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
