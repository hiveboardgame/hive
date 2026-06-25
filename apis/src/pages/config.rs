use crate::{
    components::{
        atoms::{rating::icon_for_speed, select_options::SelectOption},
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::panel::Panel,
        organisms::{
            background_color_toggle::BackgroundColorToggle,
            confirm_mode_toggle::ConfirmModeToggle,
            darkmode_toggle::{DarkModeToggle, DarkModeToggleVariant},
            preselect_toggle::PreSelectToggle,
            preview_tiles::PreviewTiles,
            takeback_conf::TakebackConf,
            tile_design_toggle::TileDesignToggle,
            tile_dots_toggle::TileDotsToggle,
            tile_rotation_toggle::TileRotationToggle,
        },
    },
    i18n::*,
};
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;
use std::str::FromStr;

#[component]
pub fn Config() -> impl IntoView {
    let i18n = use_i18n();
    let game_speed = RwSignal::new(GameSpeed::Blitz);
    let icon = move || {
        view! { <Icon attr:class="size-8 text-pillbug-teal" icon=icon_for_speed(game_speed()) /> }
    };
    let toggle = move || {
        let game_speed = game_speed();
        view! { <ConfirmModeToggle game_speed /> }
    };
    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <div class="flex flex-col gap-6 mx-auto w-full max-w-7xl">
                <div class="flex flex-col gap-1">
                    <h1 class="ui-page-title">"Settings"</h1>
                    <p class="ui-page-subtitle">"Tune board visuals and move input preferences."</p>
                </div>

                <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,24rem)]">
                    <Panel title="Board Appearance" class="h-full" body_class="space-y-4">
                        <div class="ui-setting-group">
                            <TileDesignToggle />
                        </div>
                        <div class="grid gap-4 lg:items-start lg:grid-cols-[minmax(0,1fr)_22rem]">
                            <div class="space-y-4">
                                <div class="ui-setting-group">
                                    <div class="grid gap-4 sm:grid-cols-2 sm:items-start">
                                        <TileRotationToggle />
                                        <TileDotsToggle />
                                    </div>
                                </div>
                                <div class="ui-setting-group">
                                    <div class="grid gap-4 xs:grid-cols-2 xs:items-start">
                                        <div class="flex flex-col gap-2">
                                            <p class="ui-field-label">
                                                {t!(i18n, user_config.color_scheme)}
                                            </p>
                                            <DarkModeToggle variant=DarkModeToggleVariant::Button />
                                        </div>
                                        <BackgroundColorToggle />
                                    </div>
                                </div>
                            </div>
                            <div class="flex flex-col order-first gap-2 lg:order-none ui-setting-group">
                                <p class="ui-field-label">{t!(i18n, user_config.preview)}</p>
                                <PreviewTiles />
                            </div>
                        </div>
                    </Panel>

                    <Panel title="Play Preferences" class="h-full" body_class="space-y-4">
                        <div class="ui-setting-group">
                            <label class="flex flex-col gap-1.5">
                                <span class="ui-field-label">
                                    {t!(i18n, user_config.game_speed)}
                                </span>
                                <div class="flex gap-3 items-center">
                                    {icon}
                                    <select
                                        class="ui-field-select"
                                        name="Game Speed"
                                        on:change=move |ev| {
                                            if let Ok(new_value) = GameSpeed::from_str(
                                                &event_target_value(&ev),
                                            ) {
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
                            </label>
                            <div class="mt-4">{toggle}</div>
                        </div>

                        <div class="grid gap-4 lg:grid-cols-2 xl:grid-cols-1">
                            <div class="ui-setting-group">
                                <div class="flex flex-col gap-2">
                                    <p class="ui-field-label">"Preselect"</p>
                                    <p class="ui-field-helper">
                                        "Allow selecting a piece during the opponent's turn."
                                    </p>
                                    <PreSelectToggle />
                                </div>
                            </div>
                            <div class="ui-setting-group">
                                <div class="flex flex-col gap-3">
                                    <TakebackConf />
                                    <p class="ui-field-helper">
                                        "If either player has takebacks disabled, game controls will not allow takeback requests."
                                    </p>
                                </div>
                            </div>
                        </div>
                    </Panel>
                </div>
            </div>
        </PageShell>
    }
}
