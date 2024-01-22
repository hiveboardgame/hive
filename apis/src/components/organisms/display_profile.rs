use crate::{
    components::molecules::{game_row::GameRow, user_row::UserRow},
    functions::users::get::get_user_games,
    responses::user::UserResponse,
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
    let button_styles = "z-10 w-fit flex justify-center box-content h-fit inline-block text-center hover:bg-green-300 transform transition-transform duration-300 active:scale-95 rounded-md border-cyan-500 border-2 drop-shadow-lg";
    view! {
        <div class="mt-4">
            <div class="flex flex-col items-start ml-3">
                <div class="max-w-fit">
                    <UserRow username=store_value(user().username) rating=user().rating/>
                </div>
                <p>
                    {format!("Wins: {} Draws: {} Losses {}", user().win, user().draw, user().loss)}
                </p>
            </div>
            <Transition>
                {move || {
                    let games = move || match games() {
                        Some(Ok(mut games)) => {
                            games.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                            games
                                .into_iter()
                                .partition(|game| {
                                    matches!(game.game_status, GameStatus::Finished(_))
                                })
                        }
                        _ => (Vec::new(), Vec::new()),
                    };
                    view! {
                        <div class="flex flex-col gap-2 ml-2">
                            <div class="flex gap-6 min-w-fit mb-3">
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
                        </div>
                        <div class="bg-inherit">

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
