use crate::i18n::*;
use crate::{common::TileRotation, providers::Config};
use leptos::*;

#[component]
pub fn TileRotationToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.rotation)}</p>
        <div class="flex">
            <TileRotationButton tile_rotation=TileRotation::No />
            <TileRotationButton tile_rotation=TileRotation::Yes />
        </div>
    }
}

#[component]
pub fn TileRotationButton(tile_rotation: TileRotation) -> impl IntoView {
    let i18n = use_i18n();
    let tile_rotation = store_value(tile_rotation);
    let config = expect_context::<Config>().0;
    let (_, set_cookie) = Config::get_cookie();
    let is_active = move || {
        if config().tile_rotation == tile_rotation() {
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
                                cookie.tile_rotation = tile_rotation();
                            }
                        });
                }
            >

                {match tile_rotation() {
                    TileRotation::No => t!(i18n, user_config.rotation_buttons.no).into_view(),
                    TileRotation::Yes => t!(i18n, user_config.rotation_buttons.yes).into_view(),
                }}

            </button>
        </div>
    }
}
