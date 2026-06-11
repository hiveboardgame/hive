// Wire shape for the notification-preferences settings page. Mirrors the
// editable columns on the DB model (`db_lib::models::NotificationPreferences`)
// but lives in `apis::responses` because the frontend cannot import db_lib —
// db_lib is SSR-only (Diesel proc-macros + native deps).
//
// Channel arrays come over as `Vec<String>` rather than the DB's
// `Vec<Option<String>>` (Postgres `text[]` is element-nullable but we never
// write None elements ourselves). The conversion drops any Nones a manual
// psql write might have introduced.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationPreferencesResponse {
    pub your_turn: Vec<String>,
    pub challenges: Vec<String>,
    pub game_ended: Vec<String>,
    pub tournament: Vec<String>,
    pub dms: Vec<String>,
    pub quiet_start: Option<i16>,
    pub quiet_end: Option<i16>,
    pub timezone: Option<String>,
}

#[cfg(feature = "ssr")]
impl From<db_lib::models::NotificationPreferences> for NotificationPreferencesResponse {
    fn from(p: db_lib::models::NotificationPreferences) -> Self {
        Self {
            your_turn: flatten(p.your_turn),
            challenges: flatten(p.challenges),
            game_ended: flatten(p.game_ended),
            tournament: flatten(p.tournament),
            dms: flatten(p.dms),
            quiet_start: p.quiet_start,
            quiet_end: p.quiet_end,
            timezone: p.timezone,
        }
    }
}

#[cfg(feature = "ssr")]
fn flatten(v: Vec<Option<String>>) -> Vec<String> {
    v.into_iter().flatten().collect()
}
