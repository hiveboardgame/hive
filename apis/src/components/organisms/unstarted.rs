use leptos::*;

use crate::providers::{game_state::GameStateSignal, ApiRequests};

#[component]
pub fn Unstarted(
    user_is_player: Signal<bool>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] overwrite_tw_classes: &'static str,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_id = create_read_slice(game_state.signal, |gs| gs.game_id.clone());
    let start = move |_| {
        if let Some(id) = game_id() {
            let api = ApiRequests::new();
            api.tournament_game_start(id)
        }
    };
    view! {
        <div class=if !overwrite_tw_classes.is_empty() {
            overwrite_tw_classes.to_string()
        } else {
            format!("h-full w-full col-span-8 row-span-6 {extend_tw_classes}")
        }>
            <div class="flex justify-center items-center h-full">
                <Show
                    when=user_is_player
                    fallback=move || {
                        view! {
                            <div class="">
                                "TOURNAMENT GAME NEEDS TO BE STARTED BY PLAYERS"
                            </div>
                        }
                    }
                >

                    <button on:click=start class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95">Ready</button>
                </Show>
            </div>
        </div>
    }
}
