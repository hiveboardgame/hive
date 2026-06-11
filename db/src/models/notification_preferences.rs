use crate::{db_error::DbError, schema::notification_preferences, DbConn};
use diesel::{ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Editable subset of `NotificationPreferences`. Excludes `user_id`
/// (identity, not an attribute) and `general_chat` (chat revamp parked on a
/// separate branch — surfacing it now would set us up for migration headaches
/// when that lands). Element-Nullable channel arrays match the underlying
/// Postgres `text[]` shape; the settings page only writes non-None values.
#[derive(Debug, Clone)]
pub struct NotificationPreferencesUpdate {
    pub your_turn: Vec<Option<String>>,
    pub challenges: Vec<Option<String>>,
    pub game_ended: Vec<Option<String>>,
    pub tournament: Vec<Option<String>>,
    pub dms: Vec<Option<String>>,
    pub quiet_start: Option<i16>,
    pub quiet_end: Option<i16>,
    pub timezone: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = notification_preferences)]
pub struct NewNotificationPreferences {
    pub user_id: Uuid,
}

#[derive(Queryable, Identifiable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(user_id))]
#[diesel(table_name = notification_preferences)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    // Array elements are typed Nullable<Text> because Postgres TEXT[] doesn't
    // enforce NOT NULL on elements (only on the column). Callers should filter
    // None — the CHECK constraint guarantees every non-null element is one of
    // 'push' / 'email' / 'discord'.
    pub your_turn: Vec<Option<String>>,
    pub challenges: Vec<Option<String>>,
    pub game_ended: Vec<Option<String>>,
    pub tournament: Vec<Option<String>>,
    pub general_chat: Vec<Option<String>>,
    pub dms: Vec<Option<String>>,
    pub quiet_start: Option<i16>,
    pub quiet_end: Option<i16>,
    pub timezone: Option<String>,
}

impl NotificationPreferences {
    /// Insert a defaults row at user-registration time. Called from
    /// `User::create` so every user has exactly one row.
    pub async fn create_for_user(uid: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(diesel::insert_into(notification_preferences::table)
            .values(NewNotificationPreferences { user_id: uid })
            .get_result(conn)
            .await?)
    }

    pub async fn find_for_user(uid: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(notification_preferences::table
            .find(uid)
            .first(conn)
            .await?)
    }

    /// Overwrite the editable fields for the given user. Called from the
    /// settings page save handler. Returns the row post-update so the
    /// caller can re-render with the canonical state — round-trip avoids
    /// drift between the page's local signal state and DB.
    pub async fn update_for_user(
        uid: Uuid,
        upd: NotificationPreferencesUpdate,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(diesel::update(notification_preferences::table.find(uid))
            .set((
                notification_preferences::your_turn.eq(upd.your_turn),
                notification_preferences::challenges.eq(upd.challenges),
                notification_preferences::game_ended.eq(upd.game_ended),
                notification_preferences::tournament.eq(upd.tournament),
                notification_preferences::dms.eq(upd.dms),
                notification_preferences::quiet_start.eq(upd.quiet_start),
                notification_preferences::quiet_end.eq(upd.quiet_end),
                notification_preferences::timezone.eq(upd.timezone),
            ))
            .get_result(conn)
            .await?)
    }
}
