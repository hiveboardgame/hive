use crate::{
    components::molecules::game_row::GameRow,
    pages::profile_view::{AllUserGames, ProfileGamesView},
};
use leptos::*;

#[component]
pub fn DisplayGames(tab_view: ProfileGamesView) -> impl IntoView {
    let all_games = expect_context::<AllUserGames>();
    let games = store_value(match tab_view {
        ProfileGamesView::Finished => all_games.finished,
        ProfileGamesView::Playing => all_games.playing,
    });
    let is_active = expect_context::<RwSignal<ProfileGamesView>>();
    let elem = create_node_ref::<html::Div>();
    elem.on_load(move |_| is_active.update(|v| *v = tab_view));
    view! {
        <div ref=elem class="w-full flex flex-col items-center">
            <For
                each=move || games()

                key=|game| (game.game_id)
                let:game
            >
                <GameRow game=store_value(game)/>
            </For>
        </div>
    }
}
