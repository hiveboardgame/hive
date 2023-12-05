use crate::{
    components::molecules::game_row::GameRow,
    functions::users::{get::get_user_games, user_response::UserResponse},
};
use hive_lib::game_status::GameStatus;
use leptos::*;

#[derive(Clone, PartialEq)]
pub enum TabView {
    Playing,
    Games,
}

#[component]
pub fn DisplayProfile(user: StoredValue<UserResponse>) -> impl IntoView {
    let games = Resource::once(move || get_user_games(user().username));
    let tab_view = create_rw_signal(TabView::Playing);
    let button_styles = "flex justify-center box-content h-fit inline-block text-center cursor-pointer hover:bg-green-300 rounded-md border-cyan-500 border-2 drop-shadow-lg";
    view! {
        <div class="h-full w-full grid grid-cols-6">
            <Transition>
                {move || {
                    let games = move || match games() {
                        Some(Ok(games)) => {
                            games
                                .into_iter()
                                .partition(|game| {
                                    matches!(game.game_status, GameStatus::Finished(_))
                                })
                        }
                        _ => (Vec::new(), Vec::new()),
                    };
                    view! {
                        <div class="flex flex-col gap-6 col-span-1 col-start-1 fixed">
                            <div class="flex gap-6">
                                <button
                                    class=move || {
                                        let side = match tab_view() {
                                            TabView::Playing => "bg-green-500",
                                            TabView::Games => "",
                                        };
                                        format!("{button_styles} {side}")
                                    }

                                    on:click=move |_| {
                                        tab_view.update(move |v| *v = TabView::Playing);
                                    }
                                >

                                    "Playing "
                                    {games().1.len()}
                                </button>
                                <button
                                    class=move || {
                                        let side = match tab_view() {
                                            TabView::Playing => "",
                                            TabView::Games => "bg-green-500",
                                        };
                                        format!("{button_styles} {side}")
                                    }

                                    on:click=move |_| {
                                        tab_view.update(move |v| *v = TabView::Games);
                                    }
                                >

                                    "Finished Games "
                                    {games().0.len()}
                                </button>
                            </div>
                            <p>"🟢 " {user().username}</p>
                            <p>"Rating: " {user().rating}</p>
                            <p>"Wins: " {user().win}</p>
                            <p>"Draws: " {user().draw}</p>
                            <p>"Losses " {user().loss}</p>

                        </div>
                        <div class="bg-white dark:bg-gray-900 z-40 gap-6 col-span-5 col-start-2">

                            <Show
                                when=move || { tab_view() == TabView::Playing }
                                fallback=move || {
                                    view! {
                                        <div class="w-full flex flex-col items-center">
                                            <For
                                                each=move || { games().0 }

                                                key=|game| (game.game_id)
                                                let:game
                                            >
                                                <GameRow game=store_value(game)/>
                                            </For>
                                        </div>
                                    }
                                }
                            >

                                <div class="w-full flex flex-col items-center">
                                    <For
                                        each=move || { games().1 }

                                        key=|game| (game.game_id)
                                        let:game
                                    >
                                        <GameRow game=store_value(game)/>
                                    </For>
                                </div>
                            </Show>
                        </div>
                    }
                }}

            </Transition>
        </div>
    }
}
