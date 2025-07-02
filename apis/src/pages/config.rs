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
        <div class="mx-auto max-w-4xl pt-10 px-4 sm:px-6 lg:px-8">
            <div class="mb-8 text-center">
                <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
                    "⚙️ " {t!(i18n, header.user_menu.config)}
                </h1>
                <p class="text-gray-600 dark:text-gray-400">
                    "Customize your gaming experience"
                </p>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                // Game Board & Tiles Settings
                <div class="rounded-lg shadow-lg bg-stone-300 dark:bg-slate-800 border border-stone-400 dark:border-slate-600">
                    <div class="px-6 py-4 border-b border-stone-400 dark:border-slate-600">
                        <h2 class="text-xl font-bold text-purple-600 dark:text-purple-400">
                            "🎯 Board & Tiles"
                        </h2>
                    </div>
                    <div class="p-6 space-y-6">
                        // Tile Design Settings
                        <div class="space-y-4">
                            <h3 class="font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-600 pb-2">
                                "🎨 Design Options"
                            </h3>
                            <div class="space-y-3">
                                <TileDesignToggle />
                                <TileRotationToggle />
                                <TileDotsToggle />
                            </div>
                        </div>
                        
                        // Tile Preview
                        <div class="space-y-3">
                            <h3 class="font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-600 pb-2">
                                "👁️ " {t!(i18n, user_config.preview)}
                            </h3>
                            <div class="bg-gray-100 dark:bg-gray-700 rounded-lg p-4">
                                <PreviewTiles />
                            </div>
                        </div>
                    </div>
                </div>

                // Gameplay Settings
                <div class="rounded-lg shadow-lg bg-stone-300 dark:bg-slate-800 border border-stone-400 dark:border-slate-600">
                    <div class="px-6 py-4 border-b border-stone-400 dark:border-slate-600">
                        <h2 class="text-xl font-bold text-green-600 dark:text-green-400">
                            "🎮 Gameplay"
                        </h2>
                    </div>
                    <div class="p-6 space-y-6">
                        // Game Speed Selection
                        <div class="space-y-3">
                            <h3 class="font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-600 pb-2">
                                "⚡ " {t!(i18n, user_config.game_speed)}
                            </h3>
                            <div class="flex items-center space-x-3 p-3 bg-gray-100 dark:bg-gray-700 rounded-lg">
                                {icon}
                                <select
                                    class="flex-1 px-3 py-2 bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-gray-900 dark:text-white"
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
                                        text=t!(i18n, game.speeds.bullet)
                                    />
                                    <SelectOption
                                        value=game_speed
                                        is="Blitz"
                                        text=t!(i18n, game.speeds.blitz)
                                    />
                                    <SelectOption
                                        value=game_speed
                                        is="Rapid"
                                        text=t!(i18n, game.speeds.rapid)
                                    />
                                    <SelectOption
                                        value=game_speed
                                        is="Classic"
                                        text=t!(i18n, game.speeds.classic)
                                    />
                                    <SelectOption
                                        value=game_speed
                                        is="Correspondence"
                                        text=t!(i18n, game.speeds.correspondence)
                                    />
                                    <SelectOption
                                        value=game_speed
                                        is="Untimed"
                                        text=t!(i18n, game.speeds.untimed)
                                    />
                                </select>
                            </div>
                        </div>

                        // Game Options
                        <div class="space-y-4">
                            <h3 class="font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-600 pb-2">
                                "🎯 Game Options"
                            </h3>
                            <div class="space-y-3">
                                <div class="p-3 bg-gray-100 dark:bg-gray-700 rounded-lg">
                                    <TakebackConf />
                                </div>
                                <div class="p-3 bg-gray-100 dark:bg-gray-700 rounded-lg">
                                    {toggle}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                // Display & Theme Settings
                <div class="lg:col-span-2 rounded-lg shadow-lg bg-stone-300 dark:bg-slate-800 border border-stone-400 dark:border-slate-600">
                    <div class="px-6 py-4 border-b border-stone-400 dark:border-slate-600">
                        <h2 class="text-xl font-bold text-indigo-600 dark:text-indigo-400">
                            "🎨 " {t!(i18n, user_config.color_scheme)} " & Display"
                        </h2>
                    </div>
                    <div class="p-6">
                        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between">
                            <div class="mb-4 sm:mb-0">
                                <h3 class="font-semibold text-gray-700 dark:text-gray-300 mb-2">
                                    "🌙 Theme Preference"
                                </h3>
                                <p class="text-sm text-gray-600 dark:text-gray-400">
                                    "Switch between light and dark modes for better visibility"
                                </p>
                            </div>
                            <div class="bg-gray-100 dark:bg-gray-700 rounded-lg p-4">
                                <DarkModeToggle />
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
