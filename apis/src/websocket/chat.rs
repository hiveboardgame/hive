//! In-memory recent-chat cache only. All durable chat state is in Postgres.
//!
//! This cache holds the last CHAT_RECENT_CACHE_MAX messages per (channel_type, channel_id).
//! It is only appended to on successful message persistence.

use std::{collections::HashMap, sync::RwLock};

use shared_types::ChatMessageContainer;

/// Max messages kept per channel in the recent cache.
pub const CHAT_RECENT_CACHE_MAX: usize = 50;

/// Key for the recent-messages cache: (channel_type, channel_id).
pub type ChatChannelKey = (String, String);

#[derive(Debug, Default)]
pub struct Chats {
    /// Recent messages per (channel_type, channel_id), cap at CHAT_RECENT_CACHE_MAX.
    recent_cache: RwLock<HashMap<ChatChannelKey, Vec<ChatMessageContainer>>>,
}

impl Chats {
    pub fn new() -> Self {
        Self {
            recent_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Appends one message to the channel's recent cache, keeping at most CHAT_RECENT_CACHE_MAX.
    pub fn push_recent(&self, channel_type: &str, channel_id: &str, msg: ChatMessageContainer) {
        let key = (channel_type.to_string(), channel_id.to_string());
        let mut cache = self
            .recent_cache
            .write()
            .unwrap_or_else(|error| error.into_inner());
        let entry = cache.entry(key).or_default();
        entry.push(msg);
        if entry.len() > CHAT_RECENT_CACHE_MAX {
            entry.drain(0..entry.len() - CHAT_RECENT_CACHE_MAX);
        }
    }
}
