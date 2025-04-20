use crate::components::layouts::base_layout::OrientationSignal;
use crate::i18n::*;
use crate::providers::game_state::GameStateSignal;
use crate::providers::ApiRequestsProvider;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameId;
use uuid::Uuid;

#[component]
pub fn Unstarted(
    game_id: Memo<GameId>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    user_is_player: Signal<bool>,
    ready: RwSignal<(GameId, Uuid)>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let game_state = expect_context::<GameStateSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let white = create_read_slice(game_state.signal, |gs| {
        (gs.game_response
            .as_ref()
            .map(|gr| gr.white_player.username.clone()),)
    });
    let black = create_read_slice(game_state.signal, |gs| {
        (gs.game_response
            .as_ref()
            .map(|gr| gr.black_player.username.clone()),)
    });
    let icon_for_color = move |id| {
        let icon = if ready().0 == game_id() && Some(ready().1) == id {
            icondata::AiCheckOutlined
        } else {
            icondata::IoCloseSharp
        };
        view! { <Icon icon attr:class="w-6 h-6" /> }
    };

    let start = move |_| {
        let api = api.get();
        api.tournament_game_start(game_id());
    };
    let style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex grow min-h-0 justify-center items-center h-full w-full"
        } else {
            "col-span-8 row-span-6"
        }
    };
    view! {
        <div class=style>
            <div class="flex flex-col gap-1 justify-center items-center h-full">
                <div class="flex gap-1 items-center">
                    <div class="flex gap-1 items-center">
                        {white} {move || icon_for_color(white_and_black_ids().0)}
                    </div>
                    "—"
                    <div class="flex gap-1 items-center">
                        {black} {move || icon_for_color(white_and_black_ids().1)}
                    </div>
                </div>
                <Show
                    when=user_is_player
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
