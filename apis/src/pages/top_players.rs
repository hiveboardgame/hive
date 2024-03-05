use leptos::*;
use shared_types::game_speed::GameSpeed;

use crate::components::organisms::leaderboard::Leaderboard;

#[component]
pub fn TopPlayers() -> impl IntoView {
    let leaderboards = GameSpeed::all_rated_games()
        .into_iter()
        .map(|speed| {
            view! { <Leaderboard speed=speed/> }
        })
        .collect_view();
    view! {
        <div class="pt-20 flex">
            <div class="flex flex-col md:flex-row gap-1 items-center flex-wrap w-full justify-center">
                {leaderboards}
            </div>
        </div>
    }
}
