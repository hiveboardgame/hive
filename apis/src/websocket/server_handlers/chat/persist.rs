//! Converts `ChatMessageContainer` into the form needed for DB persistence.

use chrono::{DateTime, Utc};
use db_lib::models::NewChatMessage;
use shared_types::{ChatMessageContainer, PersistentChannelKey};
use uuid::Uuid;

/// Owned form of a chat message suitable for building `db_lib::NewChatMessage`.
/// Call `as_new()` to get a reference type for `insert_chat_message`.
#[derive(Debug, Clone)]
pub struct PersistableChatMessage {
    pub channel_type: String,
    pub channel_id: String,
    pub sender_id: Uuid,
    pub username: String,
    pub body: String,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl PersistableChatMessage {
    /// Build from a container (e.g. after `container.time()` has been called).
    pub fn from_container(
        container: &ChatMessageContainer,
        channel_key: &PersistentChannelKey,
    ) -> Self {
        let turn = container.message.turn.map(|u| u as i32);
        // Preserve send-time ordering for unread/read logic even if DB persistence is delayed.
        let created_at = container.message.timestamp.unwrap_or_else(Utc::now);
        Self {
            channel_type: channel_key.channel_type.to_string(),
            channel_id: channel_key.channel_id.clone(),
            sender_id: container.message.user_id,
            username: container.message.username.clone(),
            body: container.message.message.clone(),
            turn,
            created_at,
        }
    }

    /// Build from raw parts (e.g. from REST API body).
    pub fn from_parts(
        channel_type: String,
        channel_id: String,
        sender_id: Uuid,
        username: String,
        body: String,
        turn: Option<i32>,
    ) -> Self {
        Self {
            channel_type,
            channel_id,
            sender_id,
            username,
            body,
            turn,
            created_at: Utc::now(),
        }
    }

    /// Borrow as `NewChatMessage` for use with `insert_chat_message`.
    pub fn as_new(&self) -> NewChatMessage<'_> {
        let recipient_id =
            PersistentChannelKey::from_raw(self.channel_type.as_str(), self.channel_id.as_str())
                .and_then(|channel_key| channel_key.direct_other_user_id(self.sender_id));

        NewChatMessage {
            channel_type: self.channel_type.as_str(),
            channel_id: self.channel_id.as_str(),
            sender_id: self.sender_id,
            recipient_id,
            username: self.username.as_str(),
            body: self.body.as_str(),
            turn: self.turn,
            created_at: self.created_at,
            game_id: None,
        }
    }
}
