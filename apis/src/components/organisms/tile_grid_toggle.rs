use crate::{i18n::*, providers::Config};
use leptos::prelude::*;

#[component]
pub fn TileGridToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.show_grid)}</p>
            <div class="ui-choice-group">
                <TileGridButton grid=false />
                <TileGridButton grid=true />
            </div>
        </div>
    }
}

#[component]
fn TileGridButton(grid: bool) -> impl IntoView {
    let i18n = use_i18n();
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config().tile.grid == grid;

    view! {
        <button
            class="px-2 w-full min-w-0 sm:px-4 ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            on:click=move |_| {
                set_cookie
                    .update(|c| {
                        if let Some(cookie) = c {
                            cookie.tile.grid = grid;
                        }
                    });
            }
        >
            {move || {
                if grid {
                    t_string!(i18n, user_config.grid_buttons.on)
                } else {
                    t_string!(i18n, user_config.grid_buttons.off)
                }
            }}
        </button>
    }
}
