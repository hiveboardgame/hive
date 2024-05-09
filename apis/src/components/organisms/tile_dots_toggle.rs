use crate::{common::config_options::TileDots, providers::config::config::Config};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn TileDotsToggle() -> impl IntoView {
    view! {
        <p class="text-black dark:text-white m-1">Show dots:</p>
        <div class="flex">
            <TileDotsButton tile_dots=TileDots::No/>
            <TileDotsButton tile_dots=TileDots::Yes/>
        </div>
    }
}

#[component]
pub fn TileDotsButton(tile_dots: TileDots) -> impl IntoView {
    let tile_dots = store_value(tile_dots);
    let config = expect_context::<Config>();
    let is_active = move || {
        if (config.tile_dots.preferred_tile_dots)() == tile_dots() {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        }
    };

    view! {
        <ActionForm
            action=config.tile_dots.action
            class="m-1 inline-flex items-center border border-transparent text-base font-medium rounded-md shadow justify-center cursor-pointer"
        >
            <input type="hidden" name="tile_dots" value=tile_dots().to_string()/>

            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

                type="submit"
            >
                {tile_dots().to_string()}
            </button>
        </ActionForm>
    }
}
