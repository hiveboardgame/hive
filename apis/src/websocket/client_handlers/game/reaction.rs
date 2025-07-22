use crate::{
    common::GameActionResponse,
    providers::{AlertType, AlertsContext, AuthContext, SoundType, Sounds},
};
use crate::{
    providers::{games::GamesSignal, UpdateNotifier},
    responses::GameResponse,
};
use hive_lib::GameControl;
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use shared_types::{GameStart, TimeMode};

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

            let location = use_location();
            let current_path = location.pathname.get_untracked();
            let game_path = format!("/game/{}", gar.game.game_id);

            if current_path.starts_with(&game_path) {
                let navigate = use_navigate();
                navigate("/", Default::default());
            }
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

pub fn handle_new_game(game_response: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    let should_navigate = if game_response.game_start != GameStart::Ready {
        let sounds = expect_context::<Sounds>();
        games.own_games_add(game_response.to_owned());
        sounds.play_sound(SoundType::NewGame);
        match game_response.time_mode {
            TimeMode::RealTime => true,
            TimeMode::Untimed => game_response.white_player.bot || game_response.black_player.bot,
            TimeMode::Correspondence => {
                //TODO: fix  correspondence and untimed auto-start
                false
            }
        }
    } else {
        false
    };
    if should_navigate {
        let auth_context = expect_context::<AuthContext>();
        let user_uuid =
            Signal::derive(move || auth_context.user.with(|a| a.as_ref().map(|user| user.id)));
        if let Some(id) = user_uuid.get_untracked() {
            if id == game_response.white_player.uid || id == game_response.black_player.uid {
                let location = use_location();
                let current_path = location.pathname.get_untracked();
                if !current_path.starts_with("/analysis") {
                    let navigate = use_navigate();
                    navigate(
                        &format!("/game/{}", game_response.game_id),
                        Default::default(),
                    );
                }
            }
        }
    }
}
