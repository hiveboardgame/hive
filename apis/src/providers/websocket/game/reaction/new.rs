use crate::{
    providers::{
        auth_context::AuthContext, games::GamesSignal,
        navigation_controller::NavigationControllerSignal,
    },
    responses::game::GameResponse,
};

use leptos::*;
use leptos_router::use_navigate;
use shared_types::time_mode::TimeMode;

// if gar.game.finished {
//     log!("Removing finished game {}", gar.game.nanoid.clone());
//     games.own_games_remove(&gar.game.nanoid);
// } else {
//     games.own_games_add(gar.game.to_owned());
// }
// let navigation_controller = expect_context::<NavigationControllerSignal>();
// if let Some(nanoid) = navigation_controller.signal.get_untracked().nanoid {
//     if nanoid == gar.game.nanoid {
//         if game_state.signal.get_untracked().state.history.moves != gar.game.history {
//             log!("history diverged, reconstructing please report this as a bug to the developers");
//             log!(
//                 "game_state history is: {:?}",
//                 game_state.signal.get_untracked().state.history.moves
//             );
//             log!("server_message history is: {:?}", gar.game.history);
//             reset_game_state(&gar.game);
//             let timer = expect_context::<TimerSignal>();
//             timer.update_from(&gar.game);
//         }
//     }
// }

pub fn handle_new_game(game_response: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    games.own_games_add(game_response.to_owned());
    let should_navigate = match game_response.time_mode {
        TimeMode::RealTime => true,
        TimeMode::Correspondence | TimeMode::Untimed => {
            let navigation_controller = expect_context::<NavigationControllerSignal>();
            navigation_controller
                .signal
                .get_untracked()
                .nanoid
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
                    &format!("/game/{}", game_response.nanoid),
                    Default::default(),
                );
            }
        }
    }
}
