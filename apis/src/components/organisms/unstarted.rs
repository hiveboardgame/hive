use crate::i18n::*;
use crate::providers::game_state::GameStateSignal;
use crate::providers::tournaments::TournamentStateContext;
use crate::providers::ApiRequestsProvider;
use leptos::prelude::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn Unstarted(
    user_is_player: Signal<Option<(String, Uuid)>>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] overwrite_tw_classes: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateSignal>();
    let ready = expect_context::<TournamentStateContext>().ready;
    let api = expect_context::<ApiRequestsProvider>().0;
    let game_id = create_read_slice(game_state.signal, |gs| gs.game_id.clone());
    let white_id = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| (gr.white_player.uid))
    });
    let black_id = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| (gr.black_player.uid))
    });
    let white_username = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(|gr| (gr.white_player.username.clone()))
    });
    let black_username = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(|gr| (gr.black_player.username.clone()))
    });

    let white_ready = move || {
        if let (Some(white_id), Some(game_id)) = (white_id(), game_id()) {
            ready().get(&game_id).map(|r| r.contains(&white_id))
        } else {
            Some(false)
        }
        .unwrap_or(false)
    };

    let black_ready = move || {
        if let (Some(black_id), Some(game_id)) = (black_id(), game_id()) {
            ready().get(&game_id).map(|r| r.contains(&black_id))
        } else {
            Some(false)
        }
        .unwrap_or(false)
    };

    let start = move |_| {
        if let Some(id) = game_id() {
            let api = api.get();
            api.tournament_game_start(id);
        };
    };
    view! {
        <div class=if !overwrite_tw_classes.is_empty() {
            overwrite_tw_classes.to_string()
        } else {
            format!("h-full w-full col-span-8 row-span-6 {extend_tw_classes}")
        }>
            <div class="flex flex-col gap-1 justify-center items-center h-full">
                <div class="flex gap-1 items-center">
                    <div class="flex gap-1 items-center">
                        {white_username}
                        <Show
                            when=white_ready
                            fallback=|| {
                                view! { <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" /> }
                            }
                        >

                            <Icon icon=icondata::AiCheckOutlined attr:class="w-6 h-6" />
                        </Show>

                    </div>
                    "—"
                    <div class="flex gap-1 items-center">
                        {black_username}
                        <Show
                            when=black_ready
                            fallback=|| {
                                view! { <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" /> }
                            }
                        >

                            <Icon icon=icondata::AiCheckOutlined attr:class="w-6 h-6" />
                        </Show>

                    </div>
                </div>
                <Show
                    when=move || user_is_player().is_some()
                    fallback=move || {
                        view! { <div class="p-1">{t!(i18n, game.start_when.both_ready)}</div> }
                    }
                >

                    {t!(i18n, game.start_when.both_click)}
                    <button
                        on:click=start

                        class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                    >
                        Ready
                    </button>

                </Show>
            </div>
        </div>
    }
}
