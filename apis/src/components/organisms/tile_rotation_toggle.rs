use crate::{common::TileRotation, i18n::*, providers::Config};
use leptos::prelude::*;

#[component]
pub fn TileRotationToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.rotation)}</p>
            <div class="ui-choice-group">
                <TileRotationButton tile_rotation=TileRotation::No />
                <TileRotationButton tile_rotation=TileRotation::Yes />
            </div>
        </div>
    }
}

#[component]
pub fn TileRotationButton(tile_rotation: TileRotation) -> impl IntoView {
    let i18n = use_i18n();
    let tile_rotation = StoredValue::new(tile_rotation);
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config.with(|c| c.tile.rotation.clone()) == tile_rotation.get_value();
    view! {
        <button
            class="w-full ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
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
    }
}
