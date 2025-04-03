use crate::{
    providers::{games::GamesSignal, AuthContext, SoundType, Sounds},
    responses::GameResponse,
};

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use shared_types::{GameStart, TimeMode};

pub fn handle_new_game(game_response: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    let should_navigate = if game_response.game_start != GameStart::Ready {
        let sounds = expect_context::<Sounds>();
        games.own_games_add(game_response.to_owned());
        sounds.play_sound(SoundType::NewGame);
        match game_response.time_mode {
            TimeMode::RealTime => true,
            TimeMode::Correspondence | TimeMode::Untimed => {
                //TODO: fix  correspondence and untimed auto-start
                false
            }
        }
    } else {
        false
    };
    if should_navigate {
        let auth_context = expect_context::<AuthContext>();
        let user_uuid = Signal::derive(move || auth_context.user.get().map(|user| user.id));
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
