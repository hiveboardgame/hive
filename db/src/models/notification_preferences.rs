use crate::{db_error::DbError, schema::notification_preferences, DbConn};
use diesel::{Identifiable, Insertable, QueryDsl, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
