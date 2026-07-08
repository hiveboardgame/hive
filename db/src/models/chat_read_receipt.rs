use crate::schema::chat_read_receipts;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = chat_read_receipts)]
#[diesel(primary_key(user_id, channel_id))]
pub struct ChatReadReceipt {
    pub user_id: Uuid,
    pub channel_id: i64,
    pub last_read_message_id: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_read_receipts)]
pub struct NewChatReadReceipt {
    pub user_id: Uuid,
    pub channel_id: i64,
    pub last_read_message_id: i64,
    pub updated_at: DateTime<Utc>,
}
