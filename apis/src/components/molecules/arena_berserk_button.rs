use crate::{
    common::GameAction,
    i18n::*,
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
    },
};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::tournament::Format;

#[component]
pub fn ArenaBerserkButton(#[prop(optional)] compact: bool) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateStore>();
    let auth = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let user_color = game_state.user_color_as_signal(auth.identity);
    let response = game_state.game_response();
    let available = Memo::new(move |_| {
        response.with(|response| {
            let Some(game) = response.as_ref() else {
                return false;
            };
            let Some(color) = user_color.get() else {
                return false;
            };
            let opening_turn = match color {
                Color::White => game.turn == 0 && !game.white_berserked,
                Color::Black => game.turn <= 1 && !game.black_berserked,
            };
            !game.finished
                && opening_turn
                && game.tournament.as_ref().is_some_and(|tournament| {
                    tournament.format == Format::Arena && tournament.arena_berserk_enabled
                })
        })
    });
    let berserk = move |_| {
        if !available.get_untracked() {
            return;
        }
        if let Some(game_id) =
            response.with_untracked(|game| game.as_ref().map(|game| game.game_id.clone()))
        {
            api.get().game(game_id, GameAction::Berserk);
        }
    };
    let class = if compact {
        "ml-auto shrink-0 gap-1 !min-h-6 !px-2 !py-0.5 !text-xs"
    } else {
        "ui-button-md !px-3"
    };

    view! {
        <Show when=move || available.get()>
            <button
                type="button"
                on:click=berserk
                title=move || t_string!(i18n, game.arena.berserk_title).to_string()
                aria-label=move || t_string!(i18n, game.arena.berserk).to_string()
                class=format!(
                    "text-amber-900 bg-amber-100 border dark:text-amber-300 hover:bg-amber-200 ui-button border-amber-400/60 dark:border-amber-400/40 dark:bg-amber-400/15 dark:hover:bg-amber-400/25 {class}",
                )
            >
                <Icon icon=icondata_bs::BsLightningFill attr:class="size-3.5" />
                {t!(i18n, game.arena.berserk)}
            </button>
        </Show>
    }
}
