//! In-memory recent-chat cache only. All durable chat state is in Postgres.
//!
//! This cache holds the last CHAT_RECENT_CACHE_MAX messages per (channel_type, channel_id).
//! It is populated only from: (1) DB when loading history on join, (2) new messages on insert.
//! Join paths try cache first, then Postgres.

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

    /// Returns a clone of the cached recent messages for the channel, if any.
    pub fn get_recent(
        &self,
        channel_type: &str,
        channel_id: &str,
    ) -> Option<Vec<ChatMessageContainer>> {
        let cache = self.recent_cache.read().unwrap();
        cache
            .get(&(channel_type.to_string(), channel_id.to_string()))
            .filter(|v| !v.is_empty())
            .cloned()
    }

    /// Stores up to CHAT_RECENT_CACHE_MAX most recent messages for the channel (replaces existing cache entry).
    pub fn put_recent(
        &self,
        channel_type: &str,
        channel_id: &str,
        messages: Vec<ChatMessageContainer>,
    ) {
        let key = (channel_type.to_string(), channel_id.to_string());
        let trimmed = if messages.len() > CHAT_RECENT_CACHE_MAX {
            messages[messages.len() - CHAT_RECENT_CACHE_MAX..].to_vec()
        } else {
            messages
        };
        let mut cache = self.recent_cache.write().unwrap();
        cache.insert(key, trimmed);
    }

    /// Appends one message to the channel's recent cache, keeping at most CHAT_RECENT_CACHE_MAX.
    pub fn push_recent(&self, channel_type: &str, channel_id: &str, msg: ChatMessageContainer) {
        let key = (channel_type.to_string(), channel_id.to_string());
        let mut cache = self.recent_cache.write().unwrap();
        let entry = cache.entry(key).or_default();
        entry.push(msg);
        if entry.len() > CHAT_RECENT_CACHE_MAX {
            entry.drain(0..entry.len() - CHAT_RECENT_CACHE_MAX);
        }
    }
}
