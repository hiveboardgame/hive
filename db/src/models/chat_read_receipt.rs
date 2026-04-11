use crate::schema::chat_read_receipts;
use chrono::{DateTime, Utc};
use diesel::Insertable;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_read_receipts)]
pub struct NewChatReadReceipt<'a> {
    pub user_id: Uuid,
    pub channel_type: &'a str,
    pub channel_id: &'a str,
    pub last_read_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}
