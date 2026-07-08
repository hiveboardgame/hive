use crate::schema::chat_messages;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = chat_messages)]
pub struct ChatMessage {
    pub id: i64,
    pub channel_id: i64,
    pub sender_id: Uuid,
    pub body: String,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_messages)]
pub struct NewChatMessage<'a> {
    pub channel_id: i64,
    pub sender_id: Uuid,
    pub body: &'a str,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
}
