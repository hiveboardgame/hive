use crate::i18n::*;
use crate::{common::TileDots, providers::Config};
use leptos::prelude::*;
#[component]
pub fn TileDotsToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.show_dots)}</p>
        <div class="flex flex-wrap">
            <TileDotsButton tile_dots=TileDots::No />
            <TileDotsButton tile_dots=TileDots::Angled />
            <TileDotsButton tile_dots=TileDots::Vertical />
        </div>
    }
}

#[component]
pub fn TileDotsButton(tile_dots: TileDots) -> impl IntoView {
    let i18n = use_i18n();
    let tile_dots = Signal::derive(move || tile_dots.clone());
    let Config(config, set_cookie) = expect_context();
    let is_active = move || {
        if config().tile.dots == tile_dots() {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    };

    view! {
        <div class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer">
            <button
                class=move || {
                    format!(
                        "size-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

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
        </div>
    }
}
