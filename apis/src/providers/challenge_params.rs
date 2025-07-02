use crate::common::TimeParams;
use codee::{binary::MsgpackSerdeCodec, string::Base64};
use cookie::SameSite;
use leptos::prelude::*;
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

const CHALENGE_PARAMS_COOKIE: &str = "challenge_params";
const CONF_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

#[derive(Debug, Clone, Store, Serialize, Deserialize)]
pub struct ChallengeParams {
    pub rated: bool,
    pub with_expansions: bool,
    pub is_public: bool,
    pub upper_slider: i32,
    pub lower_slider: i32,
    pub time_signals: TimeParams,
}

impl ChallengeParams {
    pub fn new() -> Self {
        let upper_slider = 550;
        let lower_slider = -550;
        let time_signals = TimeParams::default();
        Self {
            rated: true,
            with_expansions: true,
            is_public: true,
            upper_slider,
            lower_slider,
            time_signals,
        }
    }
}

impl Default for ChallengeParams {
    fn default() -> Self {
        Self::new()
    }
}
pub fn challenge_params_cookie() -> (
    Signal<Option<ChallengeParams>>,
    WriteSignal<Option<ChallengeParams>>,
) {
    let (cookie, set_cookie) = use_cookie_with_options::<ChallengeParams, Base64<MsgpackSerdeCodec>>(
        CHALENGE_PARAMS_COOKIE,
        UseCookieOptions::<ChallengeParams, _, _>::default()
            .same_site(SameSite::Lax)
            .secure(true)
            .max_age(CONF_MAX_AGE)
            .default_value(Some(ChallengeParams::default()))
            .path("/"),
    );
    (cookie, set_cookie)
}
pub fn provide_challenge_params() {
    let (cookie, _) = challenge_params_cookie();
    if let Some(cookie) = cookie.get_untracked() {
        provide_context(Store::new(cookie));
    } else {
        provide_context(Store::new(ChallengeParams::default()));
    }
}
