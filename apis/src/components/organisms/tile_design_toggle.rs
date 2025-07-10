use crate::i18n::*;
use crate::{common::TileDesign, providers::Config};
use lazy_static::lazy_static;
use leptos::prelude::*;
use leptos_use::use_window;

lazy_static! {
    pub static ref NOT_APPLE: bool =
        if let Some(Ok(user_agent)) = use_window().navigator().map(|n| n.user_agent()) {
            !(user_agent.contains("Safari")
                || user_agent.contains("iPhone")
                || user_agent.contains("iPad")
                || user_agent.contains("iPod"))
        } else {
            true
        };
}

#[component]
pub fn TileDesignToggle() -> impl IntoView {
    let i18n = use_i18n();
    let good_software = RwSignal::new(false);
    Effect::new(move |_| good_software.update(|b| *b = *NOT_APPLE));
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.piece_style)}</p>
        <div class="flex flex-wrap">
            <TileDesignButton tile_design=TileDesign::Official />
            <TileDesignButton tile_design=TileDesign::Flat />
            <Show when=good_software>
                <TileDesignButton tile_design=TileDesign::ThreeD />
            </Show>
            <TileDesignButton tile_design=TileDesign::HighContrast />
            <TileDesignButton tile_design=TileDesign::Community />
            <TileDesignButton tile_design=TileDesign::Pride />
        </div>
    }
}

#[component]
pub fn TileDesignButton(tile_design: TileDesign) -> impl IntoView {
    let tile_design = Signal::derive(move || tile_design.clone());
    let Config(config, set_cookie) = expect_context();
    let is_active = move || {
        if config().tile.design == tile_design() {
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
                        "w-full h-full transform transition-transform duration-300 active:scale-95 font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

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
        </div>
    }
}

#[component]
pub fn TilePreview(tile_design: TileDesign) -> impl IntoView {
    let design_folder = match tile_design {
        TileDesign::Official => "official",
        TileDesign::Flat => "flat",
        TileDesign::ThreeD => "3d",
        TileDesign::HighContrast => "high-contrast",
        TileDesign::Community => "community",
        TileDesign::Pride => "lgbtq",
    };

    // For other styles, show background + piece
    let piece_name = match tile_design {
        TileDesign::ThreeD | TileDesign::Pride => "whiteAnt.svg", // 3D and Pride folders use different naming
        _ => "Ant.svg",                       // All other folders use standard naming
    };

    view! {
        <div class="relative w-12 h-12">
            // Background tile - white for light mode, black for dark mode
            <img
                src=format!("/assets/tiles/{}/white.svg", design_folder)
                alt="White Background"
                class="absolute inset-0 w-full h-full object-contain dark:hidden"
            />
            <img
                src=format!("/assets/tiles/{}/black.svg", design_folder)
                alt="Black Background"
                class="absolute inset-0 w-full h-full object-contain hidden dark:block"
            />

            // Piece overlay
            <img
                src=format!("/assets/tiles/{}/{}", design_folder, piece_name)
                alt=format!("{:?} Ant", tile_design)
                class="absolute inset-0 w-full h-full object-contain"
            />
        </div>
    }
}
