use crate::common::{
    MoveConfirm,
    StackNumberStyle,
    TileDesign,
    TileDots,
    TileRotation,
    SECURE_COOKIES,
};
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
    #[serde(default)]
    pub allow_premove: bool,
    #[serde(default)]
    pub drag_and_drop: bool,
    #[serde(default)]
    pub show_stack_height: bool,
    #[serde(default)]
    pub show_reserve_counts: bool,
    #[serde(default)]
    pub stack_number_style: StackNumberStyle,
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
            allow_premove: false,
            drag_and_drop: false,
            show_stack_height: false,
            show_reserve_counts: false,
            stack_number_style: StackNumberStyle::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Config(pub Signal<ConfigOpts>, pub WriteSignal<Option<ConfigOpts>>);

impl Config {
    pub fn show_stack_height(&self) -> Signal<bool> {
        let config = self.0;
        Signal::derive(move || config.with(|config| config.show_stack_height))
    }

    pub fn show_any_numbers(&self) -> Signal<bool> {
        let config = self.0;
        Signal::derive(move || {
            config.with(|config| config.show_stack_height || config.show_reserve_counts)
        })
    }

    pub fn show_reserve_counts(&self) -> Signal<bool> {
        let config = self.0;
        Signal::derive(move || config.with(|config| config.show_reserve_counts))
    }
}

pub fn provide_config() {
    let (cookie, set_cookie) = use_cookie_with_options::<ConfigOpts, Base64<MsgpackSerdeCodec>>(
        USER_CONFIG_COOKIE,
        UseCookieOptions::<ConfigOpts, _, _>::default()
            .same_site(SameSite::Lax)
            .secure(SECURE_COOKIES)
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
    struct ConfigBeforeStackNumbers {
        confirm_mode: HashMap<GameSpeed, MoveConfirm>,
        tile: TileOptions,
        prefers_sound: bool,
        prefers_dark: bool,
        allow_preselect: bool,
        video_dismissed: bool,
        allow_premove: bool,
        drag_and_drop: bool,
    }

    #[test]
    fn cookie_saved_before_stack_numbers_decodes_with_them_off() {
        let defaults = ConfigOpts::default();
        let old_cookie = MsgpackSerdeCodec::encode(&ConfigBeforeStackNumbers {
            confirm_mode: defaults.confirm_mode.clone(),
            tile: defaults.tile.clone(),
            prefers_sound: true,
            prefers_dark: true,
            allow_preselect: true,
            video_dismissed: true,
            allow_premove: true,
            drag_and_drop: true,
        })
        .expect("old config encodes");

        let decoded: ConfigOpts =
            MsgpackSerdeCodec::decode(&old_cookie).expect("old cookie still decodes");

        assert!(decoded.drag_and_drop);
        assert!(!decoded.show_stack_height);
        assert!(!decoded.show_reserve_counts);
        assert_eq!(decoded.stack_number_style, StackNumberStyle::Circles);
    }
}
