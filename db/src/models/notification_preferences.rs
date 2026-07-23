use crate::{db_error::DbError, schema::notification_preferences, DbConn};
use diesel::{
    ExpressionMethods,
    Identifiable,
    Insertable,
    PgArrayExpressionMethods,
    QueryDsl,
    Queryable,
    Selectable,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NotificationPreferencesUpdate {
    pub your_turn: Vec<Option<String>>,
    pub challenges: Vec<Option<String>>,
    pub game_ended: Vec<Option<String>>,
    pub tournament: Vec<Option<String>>,
    pub schedules: Vec<Option<String>>,
    pub general_chat: Vec<Option<String>>,
    pub dms: Vec<Option<String>>,
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
    pub your_turn: Vec<Option<String>>,
    pub challenges: Vec<Option<String>>,
    pub game_ended: Vec<Option<String>>,
    pub tournament: Vec<Option<String>>,
    pub schedules: Vec<Option<String>>,
    pub general_chat: Vec<Option<String>>,
    pub dms: Vec<Option<String>>,
}

impl NotificationPreferences {
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
                notification_preferences::schedules.eq(upd.schedules),
                notification_preferences::general_chat.eq(upd.general_chat),
                notification_preferences::dms.eq(upd.dms),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn user_ids_with_general_chat_channel(
        channel: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        Ok(notification_preferences::table
            .filter(
                notification_preferences::general_chat.contains(vec![Some(channel.to_string())]),
            )
            .select(notification_preferences::user_id)
            .load(conn)
            .await?)
    }
}
