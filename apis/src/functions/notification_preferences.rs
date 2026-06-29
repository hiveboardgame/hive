use crate::responses::NotificationPreferencesResponse;
use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
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

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn set_notification_preferences(
    payload: NotificationPreferencesResponse,
) -> Result<NotificationPreferencesResponse, ServerFnError> {
    use crate::{
        functions::{auth::identity::uuid, db::pool},
        notifications::channel::Channel,
    };
    use db_lib::{
        get_conn,
        models::{NotificationPreferences, NotificationPreferencesUpdate},
    };
    use shared_types::NotificationCategory;
    use std::str::FromStr;

    let user_id = uuid().await?;

    for category in NotificationCategory::ALL {
        for c in payload.channels(category) {
            if Channel::from_str(c).is_err() {
                return Err(ServerFnError::new(format!(
                    "invalid channel '{c}' for event {}",
                    category.column()
                )));
            }
        }
    }
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let dedup = |v: Vec<String>| {
        let mut seen = std::collections::HashSet::new();
        v.into_iter()
            .filter(|c| seen.insert(c.clone()))
            .map(Some)
            .collect::<Vec<_>>()
    };
    let upd = NotificationPreferencesUpdate {
        your_turn: dedup(payload.your_turn),
        challenges: dedup(payload.challenges),
        game_ended: dedup(payload.game_ended),
        tournament: dedup(payload.tournament),
        schedules: dedup(payload.schedules),
        dms: dedup(payload.dms),
    };
    let updated = NotificationPreferences::update_for_user(user_id, upd, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(updated.into())
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn send_test_push() -> Result<(), ServerFnError> {
    use crate::{
        functions::{auth::identity::uuid, db::pool},
        notifications::{notify, Event},
    };
    use db_lib::{get_conn, models::PushDevice};

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let devices = PushDevice::find_for_user(user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    if devices.is_empty() {
        return Err(ServerFnError::new(
            "no registered push devices on this account",
        ));
    }
    notify(Event::TestPush { recipient: user_id });
    Ok(())
}
