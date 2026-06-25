use crate::{common::TileDots, i18n::*, providers::Config};
use leptos::prelude::*;
#[component]
pub fn TileDotsToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.show_dots)}</p>
            <div class="ui-choice-group">
                <TileDotsButton tile_dots=TileDots::No />
                <TileDotsButton tile_dots=TileDots::Angled />
                <TileDotsButton tile_dots=TileDots::Vertical />
            </div>
        </div>
    }
}

#[component]
pub fn TileDotsButton(tile_dots: TileDots) -> impl IntoView {
    let i18n = use_i18n();
    let tile_dots = Signal::derive(move || tile_dots.clone());
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config().tile.dots == tile_dots();

    view! {
        <button
            class="px-2 w-full min-w-0 sm:px-4 ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            on:click=move |_| {
                set_cookie
                    .update(|c| {
                        if let Some(cookie) = c {
                            cookie.tile.dots = tile_dots();
                        }
                    });
            }
        >
            {move || match tile_dots() {
                TileDots::No => t_string!(i18n, user_config.dots_buttons.no),
                TileDots::Angled => t_string!(i18n, user_config.dots_buttons.angled),
                TileDots::Vertical => t_string!(i18n, user_config.dots_buttons.vertical),
            }}
        </button>
    }
}
