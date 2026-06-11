// Settings-page server fns for notification preferences. Both are
// bearer-authed via `uuid()` so the HiveGame mobile app and the SSR site
// share one code path (no separate API for each auth mode).

use crate::responses::NotificationPreferencesResponse;
use leptos::prelude::*;

#[server(client = crate::client::ApiClient)]
pub async fn get_notification_preferences() -> Result<NotificationPreferencesResponse, ServerFnError>
{
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::NotificationPreferences};

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let prefs = NotificationPreferences::find_for_user(user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(prefs.into())
}

#[server(client = crate::client::ApiClient)]
pub async fn set_notification_preferences(
    payload: NotificationPreferencesResponse,
) -> Result<NotificationPreferencesResponse, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{NotificationPreferences, NotificationPreferencesUpdate},
    };

    let user_id = uuid().await?;

    // Validate channel names defensively. The DB CHECK would reject anything
    // outside push|email|discord with an opaque diesel error — surface a
    // clear message instead. Empty arrays are valid (means "no channels").
    for (label, ch) in [
        ("your_turn", &payload.your_turn),
        ("challenges", &payload.challenges),
        ("game_ended", &payload.game_ended),
        ("tournament", &payload.tournament),
        ("dms", &payload.dms),
    ] {
        for c in ch {
            if !matches!(c.as_str(), "push" | "email" | "discord") {
                return Err(ServerFnError::new(format!(
                    "invalid channel '{c}' for event {label}"
                )));
            }
        }
    }
    if let Some(h) = payload.quiet_start {
        if !(0..24).contains(&h) {
            return Err(ServerFnError::new("quiet_start must be in 0..24"));
        }
    }
    if let Some(h) = payload.quiet_end {
        if !(0..24).contains(&h) {
            return Err(ServerFnError::new("quiet_end must be in 0..24"));
        }
    }

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let upd = NotificationPreferencesUpdate {
        your_turn: payload.your_turn.into_iter().map(Some).collect(),
        challenges: payload.challenges.into_iter().map(Some).collect(),
        game_ended: payload.game_ended.into_iter().map(Some).collect(),
        tournament: payload.tournament.into_iter().map(Some).collect(),
        dms: payload.dms.into_iter().map(Some).collect(),
        quiet_start: payload.quiet_start,
        quiet_end: payload.quiet_end,
        timezone: payload.timezone,
    };
    let updated = NotificationPreferences::update_for_user(user_id, upd, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(updated.into())
}
