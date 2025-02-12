use leptos_i18n::*;
use crate::{common::TileDots, providers::Config};
use leptos::prelude::*;
#[component]
pub fn TileDotsToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.show_dots)}</p>
        <div class="flex">
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
    let config = expect_context::<Config>().0;
    let (_, set_cookie) = Config::get_cookie();
    let is_active = move || {
        if config().tile_dots == tile_dots() {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        }
    };

    view! {
        <div class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer">
            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

                on:click=move |_| {
                    set_cookie
                        .update(|c| {
                            if let Some(cookie) = c {
                                cookie.tile_dots = tile_dots();
                            }
                        });
                }
            >

                {match tile_dots() {
                    TileDots::No => t!(i18n, user_config.dots_buttons.no).into_any(),
                    TileDots::Angled => t!(i18n, user_config.dots_buttons.angled).into_any(),
                    TileDots::Vertical => t!(i18n, user_config.dots_buttons.vertical).into_any(),
                }}

            </button>
        </div>
    }
}
