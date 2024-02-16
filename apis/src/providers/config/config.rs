use super::confirm_mode::ConfirmMode;
use super::tile_design::TileDesignConfig;
use super::tile_dots::TileDotsConfig;
use super::tiles_rotation::TileRotationConfig;
use leptos::*;

#[derive(Clone)]
pub struct Config {
    pub confirm_mode: ConfirmMode,
    pub tile_design: TileDesignConfig,
    pub tile_rotation: TileRotationConfig,
    pub tile_dots: TileDotsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            confirm_mode: ConfirmMode::new(),
            tile_dots: TileDotsConfig::new(),
            tile_design: TileDesignConfig::new(),
            tile_rotation: TileRotationConfig::new(),
        }
    }
}

pub fn provide_config() {
    provide_context(Config::default())
}
