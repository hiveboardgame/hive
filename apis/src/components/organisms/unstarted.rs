use crate::i18n::*;
use crate::providers::{ApiRequestsProvider, GameUpdater};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use shared_types::GameId;
use uuid::Uuid;

#[component]
pub fn Unstarted(
    game_id: GameId,
    white: (Option<Uuid>, Option<String>),
    black: (Option<Uuid>, Option<String>),
    user_is_player: bool,
    //trick so this signal is passed down from the parent
    ready: RwSignal<Uuid>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] overwrite_tw_classes: &'static str,
) -> impl IntoView {
    let game_id = StoredValue::new(game_id);
    let (white_id, white_username) = white;
    let (black_id, black_username) = black;
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let ready_notification = expect_context::<GameUpdater>().tournament_ready;
    let white_icon = move || {
        let icon = if Some(ready()) == white_id {
            icondata::AiCheckOutlined
        } else {
            icondata::IoCloseSharp
        };
        view! { <Icon icon attr:class="w-6 h-6" /> }
    };
    let black_icon = move || {
        let icon = if Some(ready()) == black_id {
            icondata::AiCheckOutlined
        } else {
            icondata::IoCloseSharp
        };
        view! { <Icon icon attr:class="w-6 h-6" /> }
    };

    let start = move |_| {
        let api = api.get();
        api.tournament_game_start(game_id.get_value());
    };
    Effect::watch(
        ready_notification,
        move |val, _, _| {
            let (g_id, user_id) = val.clone();
            if g_id == game_id.get_value() {
                ready.set(user_id);
            }
            let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
                move |_: ()| {
                    ready.set(Uuid::default());
                },
                30_000.0,
            );
            start(());
        },
        false,
    );
    view! {
        <div class=if !overwrite_tw_classes.is_empty() {
            overwrite_tw_classes.to_string()
        } else {
            format!("h-full w-full col-span-8 row-span-6 {extend_tw_classes}")
        }>
            <div class="flex flex-col gap-1 justify-center items-center h-full">
                <div class="flex gap-1 items-center">
                    <div class="flex gap-1 items-center">{white_username} {white_icon}</div>
                    "—"
                    <div class="flex gap-1 items-center">{black_username} {black_icon}</div>
                </div>
                <Show
                    when=move || user_is_player
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
