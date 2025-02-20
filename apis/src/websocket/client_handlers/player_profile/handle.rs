use leptos::prelude::{expect_context, Get, Set};

use crate::{
    providers::{
        games_search::ProfileGamesContext, navigation_controller::NavigationControllerSignal,
    },
    responses::UserResponse,
};

pub fn handle_player_profile(profile: UserResponse) {
    let ctx = expect_context::<ProfileGamesContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    if navi
        .profile_signal
        .get()
        .username
        .is_some_and(|v| v == profile.username)
    {
        ctx.user.set(Some(profile));
    };
}
