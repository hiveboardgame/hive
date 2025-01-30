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
    create_effect(move |_| good_software.update(|b| *b = *NOT_APPLE));
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.piece_style)}</p>
        <div class="flex">
            <TileDesignButton tile_design=TileDesign::Official />
            <TileDesignButton tile_design=TileDesign::Flat />
            <Show when=good_software>
                <TileDesignButton tile_design=TileDesign::ThreeD />
            </Show>
            <TileDesignButton tile_design=TileDesign::HighContrast />
            <TileDesignButton tile_design=TileDesign::Community />
        </div>
    }
}

#[component]
pub fn TileDesignButton(tile_design: TileDesign) -> impl IntoView {
    let i18n = use_i18n();
    let tile_design = Signal::derive(move || tile_design.clone());
    let config = expect_context::<Config>().0;
    let (_, set_cookie) = Config::get_cookie();
    let is_active = move || {
        if config().tile_design == tile_design() {
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
                                cookie.tile_design = tile_design();
                            }
                        });
                }
            >

                {match tile_design() {
                    TileDesign::Official => t!(i18n, user_config.style_buttons.official).into_view(),
                    TileDesign::Flat => t!(i18n, user_config.style_buttons.flat).into_view(),
                    TileDesign::ThreeD => t!(i18n, user_config.style_buttons.three_d).into_view(),
                    TileDesign::HighContrast => t!(i18n, user_config.style_buttons.high_contrast).into_view(),
                    TileDesign::Community => t!(i18n, user_config.style_buttons.community).into_view(),
                }}

            </button>
        </div>
    }
}
