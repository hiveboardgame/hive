use crate::{components::molecules::game_previews::GamePreviews, providers::games::GamesSignal};
use leptos::prelude::*;

#[component]
pub fn Tv() -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    let live_games = Callback::new(move |_| (games.live)().live_games.into_values().collect());

    view! {
        <div class="flex flex-col items-center pt-6 lg:block 2xl:flex 2xl:flex-col 2xl:items-center">
            <div class="flex flex-col flex-wrap gap-1 justify-center items-center w-full md:flex-row lg:block 2xl:flex 2xl:items-center">
                <GamePreviews games=live_games show_time=true />
            </div>
        </div>
    }
}
