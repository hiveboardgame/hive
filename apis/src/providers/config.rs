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

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigOpts {
    pub confirm_mode: HashMap<GameSpeed, MoveConfirm>,
    pub tile_design: TileDesign,
    pub tile_rotation: TileRotation,
    pub tile_dots: TileDots,
    pub prefers_sound: bool,
    pub prefers_dark: bool,
}
#[derive(Clone, Serialize, Debug)]
pub struct Config(pub Signal<ConfigOpts>);

impl Config {
    pub fn get_cookie() -> (Signal<Option<ConfigOpts>>, WriteSignal<Option<ConfigOpts>>) {
        let exp = Utc::now()
            .checked_add_signed(chrono::Duration::days(365))
            .unwrap()
            .timestamp();
        use_cookie_with_options::<ConfigOpts, Base64<MsgpackSerdeCodec>>(
            USER_CONFIG_COOKIE,
            UseCookieOptions::default()
                .same_site(SameSite::Lax)
                .secure(true)
                .expires(exp),
        )
    }
}

impl ConfigOpts {
    //Could be removed eventually, only here to migrate old cookies
    fn new_from_legacy() -> Self {
        let (tile_dots, _) = use_cookie::<TileDots, Base64<MsgpackSerdeCodec>>("tile_dots");
        let tile_dots = tile_dots().unwrap_or_default();

        let (tile_design, _) = use_cookie::<TileDesign, FromToStringCodec>("tile_design");
        let tile_design = tile_design().unwrap_or_default();

        let (tile_rotation, _) =
            use_cookie::<TileRotation, Base64<MsgpackSerdeCodec>>("tile_rotation");
        let tile_rotation = tile_rotation().unwrap_or_default();

        let (prefers_sound, _) = use_cookie::<bool, Base64<MsgpackSerdeCodec>>("sound");
        let prefers_sound = prefers_sound().unwrap_or_default();

        let (prefers_dark, _) = use_cookie::<String, FromToStringCodec>("darkmode");
        let prefers_dark = prefers_dark().is_some_and(|v| v == "true");

        let mut confirm_mode = HashMap::new();
        for speed in GameSpeed::all().iter() {
            let move_confirm = &format!("{}_confirm_mode", speed);
            let (move_confirm, _) = use_cookie::<String, FromToStringCodec>(move_confirm);
            let move_confirm = move_confirm().and_then(|c| MoveConfirm::from_str(&c).ok());
            confirm_mode.insert(speed.clone(), move_confirm.unwrap_or_default());
        }
        Self {
            confirm_mode,
            tile_dots,
            tile_design,
            tile_rotation,
            prefers_sound,
            prefers_dark,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let (cookie, set_cookie) = Config::get_cookie();
        Self(Signal::derive(move || {
            if let Some(cookie) = cookie() {
                cookie
            } else {
                let opts = ConfigOpts::new_from_legacy();
                set_cookie(Some(opts.clone()));
                opts
            }
        }))
    }
}

pub fn provide_config() {
    provide_context(Config::default())
}
