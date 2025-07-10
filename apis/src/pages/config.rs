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
        <div class="pt-20 pb-20 mx-auto max-w-md">
            // Board and Tiles Card
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-purple-600 dark:text-purple-400">
                    "🎯 Board and Tiles"
                </h2>

                <TileDesignToggle />
                <TileRotationToggle />
                <TileDotsToggle />
            </div>

            // Preview Card
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-orange-600 dark:text-orange-400">
                    "👁️ " {t!(i18n, user_config.preview)}
                </h2>
                <PreviewTiles />
            </div>

            // Takeback Settings Card
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-blue-600 dark:text-blue-400">
                    "↩️ Allow Takebacks"
                </h2>

                <div class="p-4 mb-4 bg-amber-50 rounded-lg border border-amber-200 dark:bg-amber-900/30 dark:border-amber-700">
                    <p class="text-sm text-amber-700 dark:text-amber-300">
                        "⚠️ If either player in a game has takebacks disabled, it will no longer be possible to ask for takebacks via the game controls panel."
                    </p>
                </div>

                <TakebackConf />
            </div>

            // Game Speed & Confirmation Card
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-green-600 dark:text-green-400">
                    "⚡ Game Speed & Confirmation"
                </h2>

                <label class="mr-1">
                    <div class="flex flex-wrap items-center">
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
                </label>
                {toggle}
            </div>

            // Color Scheme Card
            <div class="px-8 pt-6 pb-8 mb-4 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-indigo-600 dark:text-indigo-400">
                    "🎨 " {t!(i18n, user_config.color_scheme)}
                </h2>
                <p class="mb-3 text-sm text-gray-700 dark:text-gray-300">
                    "Switch between light and dark themes for better visibility"
                </p>
                <DarkModeToggle />
            </div>
        </div>
    }
}
