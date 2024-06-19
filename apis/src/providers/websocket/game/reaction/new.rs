use crate::{
    providers::{
        auth_context::AuthContext, games::GamesSignal,
        navigation_controller::NavigationControllerSignal,
    },
    responses::GameResponse,
};

use leptos::*;
use leptos_router::use_navigate;
use shared_types::TimeMode;

pub fn handle_new_game(game_response: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    games.own_games_add(game_response.to_owned());
    let should_navigate = match game_response.time_mode {
        TimeMode::RealTime => true,
        TimeMode::Correspondence | TimeMode::Untimed => {
            let navigation_controller = expect_context::<NavigationControllerSignal>();
            navigation_controller
                .game_signal
                .get_untracked()
                .game_id
                .is_none()
        }
    };
    if should_navigate {
        let auth_context = expect_context::<AuthContext>();
        let user_uuid = move || match untrack(auth_context.user) {
            Some(Ok(Some(user))) => Some(user.id),
            _ => None,
        };
        if let Some(id) = user_uuid() {
            if id == game_response.white_player.uid || id == game_response.black_player.uid {
                let navigate = use_navigate();
                navigate(
                    &format!("/game/{}", game_response.game_id),
                    Default::default(),
                );
            }
        }
    }
}
