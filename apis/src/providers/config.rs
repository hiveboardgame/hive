use crate::common::{MoveConfirm, TileDesign, TileDots, TileRotation};
use chrono::Utc;
use codee::{binary::MsgpackSerdeCodec, string::Base64, string::FromToStringCodec};
use leptos::prelude::*;
use leptos_use::{use_cookie, use_cookie_with_options, SameSite, UseCookieOptions};
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;
use std::collections::HashMap;
use std::str::FromStr;

const USER_CONFIG_COOKIE: &str = "user_config";

// 1 year in milliseconds
const CONF_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ConfigOpts {
    pub confirm_mode: HashMap<GameSpeed, MoveConfirm>,
    pub tile_design: TileDesign,
    pub tile_rotation: TileRotation,
    pub tile_dots: TileDots,
    pub prefers_sound: bool,
    pub prefers_dark: bool,
}
#[derive(Clone, Debug)]
pub struct Config(pub Signal<Option<ConfigOpts>>, pub WriteSignal<Option<ConfigOpts>>);

impl Default for Config {
    fn default() -> Self {
        let (cookie, set_cookie) = use_cookie_with_options::<ConfigOpts, Base64<MsgpackSerdeCodec>>(
            USER_CONFIG_COOKIE,
            UseCookieOptions::default()
                .same_site(SameSite::Lax)
                .secure(true)
                .max_age(CONF_MAX_AGE)
                .path("/"));
        Self(cookie, set_cookie)
    }
}

pub fn provide_config() {
    provide_context(Config::default())
}
