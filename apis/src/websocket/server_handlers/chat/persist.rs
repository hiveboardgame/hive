use chrono::{DateTime, Utc};
use db_lib::{
    db_error::DbError,
    helpers::{insert_chat_message, DbChatTarget},
    models::ChatMessage,
    DbConn,
};
use shared_types::ChatMessageContainer;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PersistableChatMessage {
    pub sender_id: Uuid,
    pub body: String,
    pub turn: Option<usize>,
    pub created_at: DateTime<Utc>,
}

impl PersistableChatMessage {
    pub fn from_container(container: &ChatMessageContainer) -> Self {
        Self {
            sender_id: container.message.user_id,
            body: container.message.message.clone(),
            turn: container.message.turn,
            created_at: container.message.timestamp.unwrap_or_else(Utc::now),
        }
    }

    pub async fn insert(
        &self,
        conn: &mut DbConn<'_>,
        target: &DbChatTarget,
    ) -> Result<ChatMessage, DbError> {
        insert_chat_message(
            conn,
            self.sender_id,
            target,
            &self.body,
            self.turn,
            self.created_at,
        )
        .await
    }
}
