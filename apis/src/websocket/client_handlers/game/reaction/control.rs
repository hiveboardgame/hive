use super::handler::reset_game_state_for_takeback;
use crate::{
    common::GameActionResponse,
    providers::{
        game_state::GameStateSignal, games::GamesSignal,
        navigation_controller::NavigationControllerSignal, timer::TimerSignal, AlertType,
        AlertsContext,
    },
};
use hive_lib::{GameControl, GameResult, GameStatus};
use leptos::prelude::*;

pub fn handle_control(game_control: GameControl, gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let navigation_controller = expect_context::<NavigationControllerSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
        if gar.game.game_id == game_id {
            game_state.set_pending_gc(game_control.clone())
        }
    }
    match game_control {
        GameControl::Abort(_) => {
            games.own_games_remove(&gar.game.game_id);
            if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
                if gar.game.game_id == game_id {
                    let alerts = expect_context::<AlertsContext>();
                    alerts.last_alert.update(|v| {
                        *v = Some(AlertType::Warn(format!(
                            "{} aborted the game",
                            gar.username
                        )));
                    });
                    let navigate = leptos_router::hooks::use_navigate();
                    navigate("/", Default::default());
                }
            }
        }
        GameControl::DrawAccept(_) => {
            games.own_games_remove(&gar.game.game_id);
            if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
                if gar.game.game_id == game_id {
                    game_state.set_game_status(GameStatus::Finished(GameResult::Draw));
                    game_state.set_game_response(gar.game.clone());
                    let timer = expect_context::<TimerSignal>();
                    timer.update_from(&gar.game);
                }
            }
        }
        GameControl::Resign(color) => {
            games.own_games_remove(&gar.game.game_id);
            if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
                if gar.game.game_id == game_id {
                    game_state.set_game_status(GameStatus::Finished(GameResult::Winner(
                        color.opposite_color(),
                    )));
                    game_state.set_game_response(gar.game.clone());
                    let timer = expect_context::<TimerSignal>();
                    timer.update_from(&gar.game);
                }
            }
        }
        GameControl::TakebackAccept(_) => {
            games.own_games_add(gar.game.to_owned());
            if let Some(game_id) = navigation_controller.game_signal.get_untracked().game_id {
                if gar.game.game_id == game_id {
                    let timer = expect_context::<TimerSignal>();
                    timer.update_from(&gar.game);
                    reset_game_state_for_takeback(&gar.game);
                }
            }
        }
        GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
            games.own_games_add(gar.game.to_owned());
        }
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {}
    }
}
