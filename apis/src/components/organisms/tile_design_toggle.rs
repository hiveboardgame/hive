use crate::i18n::*;
use crate::{common::TileDesign, providers::Config};
use leptos::prelude::*;

#[component]
pub fn TileDesignToggle() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.piece_style)}</p>
        <div class="mb-2 p-2 bg-yellow-50 dark:bg-yellow-900 border border-yellow-200 dark:border-yellow-700 rounded-md">
            <p class="text-sm text-yellow-800 dark:text-yellow-200">
                "⚠️ Note: Not all browsers support all tile designs. For the best experience, we recommend using Firefox."
            </p>
        </div>
        <div class="flex flex-wrap">
            <TileDesignButton tile_design=TileDesign::Official />
            <TileDesignButton tile_design=TileDesign::Flat />
            <TileDesignButton tile_design=TileDesign::HighContrast />
            <TileDesignButton tile_design=TileDesign::ThreeD />
            <TileDesignButton tile_design=TileDesign::Community />
            <TileDesignButton tile_design=TileDesign::Pride />
            <TileDesignButton tile_design=TileDesign::Carbon />
            <TileDesignButton tile_design=TileDesign::Carbon3D />
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
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
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
        TileDesign::Carbon3D => "carbon-3d",
        TileDesign::Carbon => "carbon",
    };

    // For other styles, show background + piece
    let (light_piece_name, dark_piece_name) = match tile_design {
        TileDesign::ThreeD | TileDesign::Pride | TileDesign::Carbon3D | TileDesign::Carbon => {
            ("whiteAnt.svg", "blackAnt.svg") // whiteAnt.svg is actually dark colored, blackAnt.svg is actually light colored
        }
        _ => ("Ant.svg", "Ant.svg"), // All other folders use standard naming (no theme switching needed)
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

            // Piece overlay - black piece for light mode, white piece for dark mode
            <img
                src=format!("/assets/tiles/{}/{}", design_folder, light_piece_name)
                alt=format!("{:?} Ant (Light Mode)", tile_design)
                class="absolute inset-0 w-full h-full object-contain dark:hidden"
            />
            <img
                src=format!("/assets/tiles/{}/{}", design_folder, dark_piece_name)
                alt=format!("{:?} Ant (Dark Mode)", tile_design)
                class="absolute inset-0 w-full h-full object-contain hidden dark:block"
            />
        </div>
    }
}
