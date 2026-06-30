//! Converts `ChatMessageContainer` into the form needed for DB persistence.
use chrono::{DateTime, Utc};
use db_lib::models::NewChatMessage;
use shared_types::{ChatMessageContainer, PersistentChannelKey};
use uuid::Uuid;
/// Owned form of a chat message suitable for building `db_lib::NewChatMessage`.
/// Call `as_new()` to get a reference type for `insert_chat_message`.
#[derive(Debug, Clone)]
pub struct PersistableChatMessage {
    pub channel_key: PersistentChannelKey,
    pub sender_id: Uuid,
    pub username: String,
    pub body: String,
    pub turn: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub game_id: Option<Uuid>,
}

impl PersistableChatMessage {
    /// Build from a container (e.g. after `container.time()` has been called).
    pub fn from_container(
        container: &ChatMessageContainer,
        channel_key: &PersistentChannelKey,
        game_id: Option<Uuid>,
    ) -> Self {
        // Drop an out-of-range (or maliciously huge) turn instead of wrapping
        // it into a garbage i32 via `as`.
        let turn = container.message.turn.and_then(|u| i32::try_from(u).ok());
        // Preserve send-time ordering for unread/read logic even if DB persistence is delayed.
        let created_at = container.message.timestamp.unwrap_or_else(Utc::now);
        Self {
            channel_key: channel_key.clone(),
            sender_id: container.message.user_id,
            username: container.message.username.clone(),
            body: container.message.message.clone(),
            turn,
            created_at,
            game_id,
        }
    }

    /// Borrow as `NewChatMessage` for use with `insert_chat_message`.
    pub fn as_new(&self) -> NewChatMessage<'_> {
        let recipient_id = self.channel_key.direct_other_user_id(self.sender_id);

        NewChatMessage {
            channel_type: self.channel_key.channel_type.as_str(),
            channel_id: self.channel_key.channel_id.as_str(),
            sender_id: self.sender_id,
            recipient_id,
            username: self.username.as_str(),
            body: self.body.as_str(),
            turn: self.turn,
            created_at: self.created_at,
            game_id: self.game_id,
        }
    }
}
