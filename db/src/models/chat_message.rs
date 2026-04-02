use crate::{db_error::DbError, schema::chat_messages, DbConn};
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = chat_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChatMessage {
    pub id: i64,
    pub channel_type: String,
    pub channel_id: String,
    pub sender_id: Uuid,
    pub username: String,
    pub body: String,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_messages)]
pub struct NewChatMessage<'a> {
    pub channel_type: &'a str,
    pub channel_id: &'a str,
    pub sender_id: Uuid,
    pub username: &'a str,
    pub body: &'a str,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}

impl NewChatMessage<'_> {
    pub async fn insert(self, conn: &mut DbConn<'_>) -> Result<ChatMessage, DbError> {
        diesel::insert_into(chat_messages::table)
            .values(self)
            .get_result(conn)
            .await
            .map_err(Into::into)
    }
}
