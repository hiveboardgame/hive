use crate::schema::chat_read_receipts;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = chat_read_receipts)]
#[diesel(primary_key(user_id, channel_type, channel_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChatReadReceipt {
    pub user_id: Uuid,
    pub channel_type: String,
    pub channel_id: String,
    pub last_read_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_read_receipts)]
pub struct NewChatReadReceipt<'a> {
    pub user_id: Uuid,
    pub channel_type: &'a str,
    pub channel_id: &'a str,
    pub last_read_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}
