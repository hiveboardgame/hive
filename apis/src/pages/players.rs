use crate::components::organisms::{leaderboard::Leaderboard, online_users::OnlineUsers};
use leptos::*;

#[component]
pub fn PlayersView() -> impl IntoView {
    view! {
        <div class="pt-10">
            <OnlineUsers/>
            <Leaderboard/>
        </div>
    }
}
