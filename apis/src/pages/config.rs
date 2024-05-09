use crate::components::organisms::{
    confirm_mode_toggle::ConfirmModeToggle, darkmode_toggle::DarkModeToggle,
    preview_tiles::PreviewTiles, tile_design_toggle::TileDesignToggle,
    tile_dots_toggle::TileDotsToggle, tile_rotation_toggle::TileRotationToggle,
};
use leptos::*;

#[component]
pub fn Config() -> impl IntoView {
    view! {
        <div class="flex flex-col sm:flex-row pt-10">
            <div class="m-1">
                <TileDesignToggle/>
                <TileRotationToggle/>
                <TileDotsToggle/>
                <ConfirmModeToggle/>
                <p class="text-black dark:text-white m-1">Colorscheme:</p>
                <DarkModeToggle/>
            </div>
            <div class="m-1">
                <p class="text-black dark:text-white m-1">Preview:</p>
                <PreviewTiles/>
            </div>
        </div>
    }
}
