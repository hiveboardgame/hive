use crate::components::{
    atoms::{rating::icon_for_speed, select_options::SelectOption},
    organisms::{
        confirm_mode_toggle::ConfirmModeToggle, darkmode_toggle::DarkModeToggle,
        preview_tiles::PreviewTiles, takeback_conf::TakebackConf,
        tile_design_toggle::TileDesignToggle, tile_dots_toggle::TileDotsToggle,
        tile_rotation_toggle::TileRotationToggle,
    },
};
use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;
use std::str::FromStr;

#[component]
pub fn Config() -> impl IntoView {
    let i18n = use_i18n();
    let game_speed = RwSignal::new(GameSpeed::Blitz);
    let icon = move || {
        view! { <Icon width="50" height="50" attr:class="p-2" icon=icon_for_speed(&game_speed()) /> }
    };
    let toggle = move || {
        let game_speed = game_speed();
        view! { <ConfirmModeToggle game_speed /> }
    };
    view! {
        <div class="flex flex-col pt-10 sm:flex-row">
            <div class="m-1">
                <TileDesignToggle />
                <TileRotationToggle />
                <TileDotsToggle />
                <TakebackConf />
                <label class="mr-1">
                    <div class="flex items-center">
                        {icon} <p>{t!(i18n, user_config.game_speed)}</p>
                    </div>
                    <select
                        class="bg-odd-light dark:bg-gray-700"
                        name="Game Speed"
                        on:change=move |ev| {
                            if let Ok(new_value) = GameSpeed::from_str(&event_target_value(&ev)) {
                                game_speed.update(|v| *v = new_value);
                            }
                        }
                    >

                        <SelectOption
                            value=game_speed
                            is="Bullet"
                            text=t!(i18n, game.speeds.bullet).into_any().into()
                        />
                        <SelectOption
                            value=game_speed
                            is="Blitz"
                            text=t!(i18n, game.speeds.blitz).into_any().into()
                        />
                        <SelectOption
                            value=game_speed
                            is="Rapid"
                            text=t!(i18n, game.speeds.rapid).into_any().into()
                        />
                        <SelectOption
                            value=game_speed
                            is="Classic"
                            text=t!(i18n, game.speeds.classic).into_any().into()
                        />
                        <SelectOption
                            value=game_speed
                            is="Correspondence"
                            text=t!(i18n, game.speeds.correspondence).into_any()
                        />
                        <SelectOption
                            value=game_speed
                            is="Untimed"
                            text=t!(i18n, game.speeds.untimed).into_any()
                        />
                    </select>
                </label>
                {toggle}
                <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.color_scheme)}</p>
                <DarkModeToggle />
            </div>
            <div class="m-1">
                <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.preview)}</p>
                <PreviewTiles />
            </div>
        </div>
    }
}
