pub(super) use super::super::{test_websocket, Chat, InitialHistoryStatus};
pub(super) use crate::providers::AuthIdentity;
pub(super) use chrono::{TimeZone, Utc};
pub(super) use leptos::prelude::*;
pub(super) use shared_types::{
    ChatHistoryPage,
    ChatHistoryResponse,
    ChatMessage,
    ChatMessageContainer,
    ConversationKey,
    ConversationUnreadState,
    GameId,
    TournamentId,
};
pub(super) use std::collections::HashMap;
pub(super) use uuid::Uuid;

pub(super) fn chat_with_user(user_id: Uuid) -> Chat {
    Chat::new(test_websocket(), Some(AuthIdentity::User(user_id)))
}

pub(super) fn schedule_read(chat: Chat, key: &ConversationKey, read_through_id: i64) -> bool {
    chat.read_receipts
        .try_update_value(|receipts| receipts.schedule_read(key, read_through_id))
        .expect("chat receipt state was disposed")
}

pub(super) fn begin_scheduled_read(chat: Chat, key: &ConversationKey) -> Option<i64> {
    chat.read_receipts
        .try_update_value(|receipts| receipts.begin_scheduled_read(key))
        .expect("chat receipt state was disposed")
}

pub(super) fn finish_in_flight(chat: Chat, key: &ConversationKey, read_through_id: i64) -> bool {
    chat.read_receipts
        .try_update_value(|receipts| receipts.finish_in_flight(key, read_through_id))
        .expect("chat receipt state was disposed")
}

pub(super) fn record_confirmed_read(chat: Chat, key: &ConversationKey, read_through_id: i64) {
    chat.read_receipts.update_value(|receipts| {
        receipts.record_confirmed_read(key, read_through_id);
    });
}

pub(super) fn message(id: i64, user_id: Uuid, username: &str, body: &str) -> ChatMessage {
    ChatMessage {
        id,
        user_id,
        username: username.to_string(),
        timestamp: Utc.timestamp_millis_opt(id * 1000).single().unwrap(),
        message: body.to_string(),
        turn: None,
    }
}

pub(super) fn history_page(
    start: i64,
    end: i64,
    user_id: Uuid,
    next_before_message_id: Option<i64>,
    initial_unread_count: Option<i64>,
) -> ChatHistoryPage {
    ChatHistoryPage {
        messages: (start..=end)
            .map(|id| message(id, user_id, "current", &format!("message {id}")))
            .collect(),
        next_before_message_id,
        initial_unread_count,
    }
}
