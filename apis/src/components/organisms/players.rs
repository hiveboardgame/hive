use crate::components::organisms::{leaderboard::Leaderboard, online_users::OnlineUsers};
use leptos::prelude::*;

#[component]
pub fn PlayersView() -> impl IntoView {
    view! {
        <div class="pt-2 md:pt-6 flex flex-col items-center">
            <OnlineUsers />
            // TODO: move this out to its own component "leaderboard_s_"
            <Leaderboard speed=shared_types::game_speed::GameSpeed::Correspondence />
        </div>
    }
}
