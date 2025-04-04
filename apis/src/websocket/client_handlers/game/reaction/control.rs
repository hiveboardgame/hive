use crate::{
    common::GameActionResponse,
    providers::{games::GamesSignal, AlertType, AlertsContext, UpdateNotifier},
};
use hive_lib::GameControl;
use leptos::prelude::*;

pub fn handle_control(game_control: GameControl, gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let game_updater = expect_context::<UpdateNotifier>();
    game_updater.game_response.set(Some(gar.clone()));
    match game_control {
        GameControl::Abort(_) => {
            games.own_games_remove(&gar.game.game_id);
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
        GameControl::DrawAccept(_) => {
            games.own_games_remove(&gar.game.game_id);
        }
        GameControl::Resign(_) => {
            games.own_games_remove(&gar.game.game_id);
        }
        GameControl::TakebackAccept(_) => {
            games.own_games_add(gar.game.to_owned());
        }
        GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
            games.own_games_add(gar.game.to_owned());
        }
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {}
    }
}
