use leptos::*;
use shared_types::GameSpeed;

use crate::components::{molecules::banner::Banner, organisms::leaderboard::Leaderboard};

#[component]
pub fn TopPlayers() -> impl IntoView {
    let leaderboards = GameSpeed::all_rated_games()
        .into_iter()
        .map(|speed| {
            view! { <Leaderboard speed=speed/> }
        })
        .collect_view();
    view! {
        <div class="flex flex-col items-center pt-20">
            <Banner title="Top Rated Players".into_view() extend_tw_classes="w-10/12"/>
            <div class="flex flex-col flex-wrap gap-1 justify-center items-center w-full md:flex-row">
                {leaderboards}
            </div>
        </div>
    }
}
