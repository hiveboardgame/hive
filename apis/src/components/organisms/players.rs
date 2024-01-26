use crate::components::organisms::{leaderboard::Leaderboard, online_users::OnlineUsers};
use leptos::*;

#[component]
pub fn PlayersView() -> impl IntoView {
    view! {
        <div class="pt-2 md:pt-6 flex flex-col items-center">
            <OnlineUsers/>
            <Leaderboard/>
        </div>
    }
}
