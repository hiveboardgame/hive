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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TileOptions {
    pub design: TileDesign,
    pub rotation: TileRotation,
    pub dots: TileDots,
}

impl TileOptions {
    pub fn is_three_d(&self) -> bool {
        self.design == TileDesign::ThreeD
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigOpts {
    pub confirm_mode: HashMap<GameSpeed, MoveConfirm>,
    pub tile: TileOptions,
    pub prefers_sound: bool,
    pub prefers_dark: bool,
}

impl Default for ConfigOpts {
    fn default() -> Self {
        Self {
            confirm_mode: HashMap::from([
                (GameSpeed::Bullet, MoveConfirm::Single),
                (GameSpeed::Blitz, MoveConfirm::Single),
                (GameSpeed::Rapid, MoveConfirm::Double),
                (GameSpeed::Classic, MoveConfirm::Double),
                (GameSpeed::Correspondence, MoveConfirm::Double),
                (GameSpeed::Untimed, MoveConfirm::Double),
            ]),
            tile: TileOptions::default(),
            prefers_sound: false,
            prefers_dark: false,
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
