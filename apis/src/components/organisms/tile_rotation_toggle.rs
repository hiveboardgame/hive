use crate::i18n::*;
use crate::{common::TileRotation, providers::Config};
use leptos::prelude::*;

#[component]
pub fn TileRotationToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.rotation)}</p>
        <div class="flex flex-wrap">
            <TileRotationButton tile_rotation=TileRotation::No />
            <TileRotationButton tile_rotation=TileRotation::Yes />
        </div>
    }
}

#[component]
pub fn TileRotationButton(tile_rotation: TileRotation) -> impl IntoView {
    let i18n = use_i18n();
    let tile_rotation = StoredValue::new(tile_rotation);
    let Config(config, set_cookie) = expect_context();
    let is_active = move || {
        if config().tile.rotation == tile_rotation.get_value() {
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
                                cookie.tile.rotation = tile_rotation.get_value();
                            }
                        });
                }
            >

                {move || match tile_rotation.get_value() {
                    TileRotation::No => t_string!(i18n, user_config.rotation_buttons.no),
                    TileRotation::Yes => t_string!(i18n, user_config.rotation_buttons.yes),
                }}

            </button>
        </div>
    }
}
