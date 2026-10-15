use crate::{common::TileDesign, i18n::*, providers::Config};
use leptos::prelude::*;
use leptos_i18n::I18nContext;

#[component]
pub fn TileDesignToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.piece_style)}</p>
            <p class="ui-warning-notice">
                {move || t_string!(i18n, user_config.tile_design_browser_notice)}
            </p>
            <div class="ui-choice-group">
                <TileDesignButton tile_design=TileDesign::Official />
                <TileDesignButton tile_design=TileDesign::Flat />
                <TileDesignButton tile_design=TileDesign::HighContrast />
                <TileDesignButton tile_design=TileDesign::ThreeD />
                <TileDesignButton tile_design=TileDesign::Community />
                <TileDesignButton tile_design=TileDesign::Pride />
                <TileDesignButton tile_design=TileDesign::Carbon />
                <TileDesignButton tile_design=TileDesign::Carbon3D />
            </div>
        </div>
    }
}

#[component]
pub fn TileDesignButton(tile_design: TileDesign) -> impl IntoView {
    let tile_design = Signal::derive(move || tile_design.clone());
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config().tile.design == tile_design();

    view! {
        <button
            class="ui-choice ui-choice-tile"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            on:click=move |_| {
                set_cookie
                    .update(|c| {
                        if let Some(cookie) = c {
                            cookie.tile.design = tile_design();
                        }
                    });
            }
        >
            <TilePreview tile_design=tile_design() />
        </button>
    }
}

#[component]
pub fn TilePreview(tile_design: TileDesign) -> impl IntoView {
    let i18n = use_i18n();
    let design_folder = match tile_design {
        TileDesign::Official => "official",
        TileDesign::Flat => "flat",
        TileDesign::ThreeD => "3d",
        TileDesign::HighContrast => "high-contrast",
        TileDesign::Community => "community",
        TileDesign::Pride => "lgbtq",
        TileDesign::Carbon3D => "carbon-3d",
        TileDesign::Carbon => "carbon",
    };

    let (light_piece_name, dark_piece_name) = match tile_design {
        TileDesign::ThreeD | TileDesign::Pride | TileDesign::Carbon3D | TileDesign::Carbon => {
            ("whiteAnt.svg", "blackAnt.svg")
        }
        _ => ("Ant.svg", "Ant.svg"),
    };

    let tile_design = StoredValue::new(tile_design);
    view! {
        <div class="relative size-12">
            <img
                src=format!("/assets/tiles/{}/white.svg", design_folder)
                alt=move || t_string!(i18n, user_config.white_background)
                class="object-contain absolute inset-0 dark:hidden size-full"
            />
            <img
                src=format!("/assets/tiles/{}/black.svg", design_folder)
                alt=move || t_string!(i18n, user_config.black_background)
                class="hidden object-contain absolute inset-0 dark:block size-full"
            />

            <img
                src=format!("/assets/tiles/{}/{}", design_folder, light_piece_name)
                alt=move || {
                    t_string!(
                        i18n, user_config.ant_light_preview, design = tile_design.with_value(|design| tile_design_label(i18n, design))
                    )
                        .to_string()
                }
                class="object-contain absolute inset-0 dark:hidden size-full"
            />
            <img
                src=format!("/assets/tiles/{}/{}", design_folder, dark_piece_name)
                alt=move || {
                    t_string!(
                        i18n, user_config.ant_dark_preview, design = tile_design.with_value(|design| tile_design_label(i18n, design))
                    )
                        .to_string()
                }
                class="hidden object-contain absolute inset-0 dark:block size-full"
            />
        </div>
    }
}

fn tile_design_label(i18n: I18nContext<Locale, I18nKeys>, design: &TileDesign) -> &'static str {
    match design {
        TileDesign::Official => t_string!(i18n, user_config.tile_design.official),
        TileDesign::Flat => t_string!(i18n, user_config.tile_design.flat),
        TileDesign::ThreeD => t_string!(i18n, user_config.tile_design.three_d),
        TileDesign::HighContrast => t_string!(i18n, user_config.tile_design.high_contrast),
        TileDesign::Community => t_string!(i18n, user_config.tile_design.community),
        TileDesign::Pride => t_string!(i18n, user_config.tile_design.pride),
        TileDesign::Carbon3D => t_string!(i18n, user_config.tile_design.carbon_three_d),
        TileDesign::Carbon => t_string!(i18n, user_config.tile_design.carbon),
    }
}
