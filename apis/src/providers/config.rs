use crate::common::{MoveConfirm, TileDesign, TileDots, TileRotation};
use codee::{binary::MsgpackSerdeCodec, string::Base64};
use leptos::prelude::*;
use leptos_use::{use_cookie_with_options, SameSite, UseCookieOptions};
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;
use std::collections::HashMap;

const USER_CONFIG_COOKIE: &str = "user_config";

// 1 year in milliseconds
const CONF_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

// Background color constants
const LIGHT_DEFAULT_BG: &str = "#edebe9"; // board-dawn color (light mode)
const DARK_DEFAULT_BG: &str = "#47545a"; // board-twilight color (dark mode)

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TileOptions {
    pub design: TileDesign,
    pub rotation: TileRotation,
    pub dots: TileDots,
    pub background_color: Option<String>,
    #[serde(default)]
    pub grid: bool,
}

impl TileOptions {
    pub fn is_three_d(&self) -> bool {
        self.design == TileDesign::ThreeD
    }

    pub fn is_using_custom_background(&self, is_dark_mode: bool) -> bool {
        match &self.background_color {
            None => false,
            Some(color) => {
                let current_theme_default = if is_dark_mode {
                    DARK_DEFAULT_BG
                } else {
                    LIGHT_DEFAULT_BG
                };
                color != current_theme_default
            }
        }
    }

    pub fn get_effective_background_color(&self, is_dark_mode: bool) -> String {
        self.background_color
            .clone()
            .unwrap_or_else(|| Self::get_theme_default_background_color(is_dark_mode))
    }

    pub fn get_theme_default_background_color(is_dark_mode: bool) -> String {
        if is_dark_mode {
            DARK_DEFAULT_BG.to_string()
        } else {
            LIGHT_DEFAULT_BG.to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConfigOpts {
    pub confirm_mode: HashMap<GameSpeed, MoveConfirm>,
    pub tile: TileOptions,
    pub prefers_sound: bool,
    pub prefers_dark: bool,
    pub allow_preselect: bool,
    #[serde(default)]
    pub video_dismissed: bool,
}

impl Default for ConfigOpts {
    fn default() -> Self {
        Self {
            confirm_mode: HashMap::from([
                (GameSpeed::Bullet, MoveConfirm::Single),
                (GameSpeed::Blitz, MoveConfirm::Double),
                (GameSpeed::Rapid, MoveConfirm::Double),
                (GameSpeed::Classic, MoveConfirm::Double),
                (GameSpeed::Correspondence, MoveConfirm::Double),
                (GameSpeed::Untimed, MoveConfirm::Double),
            ]),
            tile: TileOptions::default(),
            prefers_sound: false,
            prefers_dark: false,
            allow_preselect: false,
            video_dismissed: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Config(pub Signal<ConfigOpts>, pub WriteSignal<Option<ConfigOpts>>);

pub fn provide_config() {
    let (cookie, set_cookie) = use_cookie_with_options::<ConfigOpts, Base64<MsgpackSerdeCodec>>(
        USER_CONFIG_COOKIE,
        UseCookieOptions::<ConfigOpts, _, _>::default()
            .same_site(SameSite::Lax)
            .secure(true)
            .max_age(CONF_MAX_AGE)
            .default_value(Some(ConfigOpts::default()))
            .path("/"),
    );
    let cookie = Signal::derive(move || cookie().unwrap_or_default());
    provide_context(Config(cookie, set_cookie));
}

#[cfg(test)]
mod tests {
    use super::*;
    use codee::{Decoder, Encoder};

    #[derive(Serialize)]
    struct TileOptionsBeforeGrid {
        design: TileDesign,
        rotation: TileRotation,
        dots: TileDots,
        background_color: Option<String>,
    }

    #[derive(Serialize)]
    struct ConfigOptsBeforeGrid {
        confirm_mode: HashMap<GameSpeed, MoveConfirm>,
        tile: TileOptionsBeforeGrid,
        prefers_sound: bool,
        prefers_dark: bool,
        allow_preselect: bool,
        video_dismissed: bool,
    }

    #[test]
    fn a_cookie_from_before_the_grid_option_still_loads() {
        let old = ConfigOptsBeforeGrid {
            confirm_mode: HashMap::new(),
            tile: TileOptionsBeforeGrid {
                design: TileDesign::default(),
                rotation: TileRotation::default(),
                dots: TileDots::default(),
                background_color: Some("#123456".to_string()),
            },
            prefers_sound: true,
            prefers_dark: true,
            allow_preselect: false,
            video_dismissed: true,
        };
        let bytes = MsgpackSerdeCodec::encode(&old).expect("old cookie encodes");

        let decoded: ConfigOpts = MsgpackSerdeCodec::decode(&bytes).expect("old cookie decodes");

        assert_eq!(decoded.tile.background_color.as_deref(), Some("#123456"));
        assert!(!decoded.tile.grid);
        assert!(decoded.prefers_dark);
    }
}
