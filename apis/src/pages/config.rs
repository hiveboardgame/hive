use crate::components::{
    atoms::{rating::icon_for_speed, select_options::SelectOption},
    organisms::{
        confirm_mode_toggle::ConfirmModeToggle, darkmode_toggle::DarkModeToggle,
        preview_tiles::PreviewTiles, tile_design_toggle::TileDesignToggle,
        tile_dots_toggle::TileDotsToggle, tile_rotation_toggle::TileRotationToggle,
    },
};
use leptos::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;
use std::str::FromStr;

#[component]
pub fn Config() -> impl IntoView {
    let game_speed = RwSignal::new(GameSpeed::Blitz);
    let icon = move || {
        view! { <Icon width="50" height="50" class="p-2" icon=icon_for_speed(&game_speed())/> }
    };
    let toggle = move || {
        let game_speed = game_speed();
        view! { <ConfirmModeToggle game_speed/> }
    };
    view! {
        <div class="flex flex-col pt-10 sm:flex-row">
            <div class="m-1">
                <TileDesignToggle/>
                <TileRotationToggle/>
                <TileDotsToggle/>
                <label class="mr-1">
                    <div class="flex items-center">{icon} <p>" Game speed:"</p></div>
                    <select
                        class="bg-odd-light dark:bg-gray-700"
                        name="Game Speed"
                        on:change=move |ev| {
                            if let Ok(new_value) = GameSpeed::from_str(&event_target_value(&ev)) {
                                game_speed.update(|v| *v = new_value);
                            }
                        }
                    >

                        <SelectOption value=game_speed is="Bullet"/>
                        <SelectOption value=game_speed is="Blitz"/>
                        <SelectOption value=game_speed is="Rapid"/>
                        <SelectOption value=game_speed is="Classic"/>
                        <SelectOption value=game_speed is="Correspondence"/>
                        <SelectOption value=game_speed is="Untimed"/>
                    </select>
                </label>
                {toggle}
                <p class="m-1 text-black dark:text-white">Colorscheme:</p>
                <DarkModeToggle/>
            </div>
            <div class="m-1">
                <p class="m-1 text-black dark:text-white">Preview:</p>
                <PreviewTiles/>
            </div>
        </div>
    }
}
