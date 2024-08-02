use crate::{common::TileDesign, providers::Config};
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::ActionForm;
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
    let good_software = RwSignal::new(false);
    create_effect(move |_| good_software.update(|b| *b = *NOT_APPLE));
    view! {
        <p class="m-1 text-black dark:text-white">Piece style:</p>
        <div class="flex">
            <TileDesignButton tile_design=TileDesign::Official/>
            <TileDesignButton tile_design=TileDesign::Flat/>
            <Show when=good_software>
                <TileDesignButton tile_design=TileDesign::ThreeD/>
            </Show>
        </div>
    }
}

#[component]
pub fn TileDesignButton(tile_design: TileDesign) -> impl IntoView {
    let tile_design = store_value(tile_design);
    let config = expect_context::<Config>();
    let is_active = move || {
        if (config.tile_design.preferred_tile_design)() == tile_design() {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        }
    };

    view! {
        <ActionForm
            action=config.tile_design.action
            class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer"
        >
            <input type="hidden" name="tile_design" value=tile_design().to_string()/>
            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

                type="submit"
            >
                {tile_design().to_string()}
            </button>
        </ActionForm>
    }
}
