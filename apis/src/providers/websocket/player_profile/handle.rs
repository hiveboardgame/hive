use leptos::{expect_context, SignalSet};

use crate::{pages::profile_view::ProfileGamesContext, responses::UserResponse};

pub fn handle_player_profile(profile: UserResponse) {
    let ctx = expect_context::<ProfileGamesContext>();
    ctx.user.set(Some(profile));
}
