//! In-memory recent-chat cache only. All durable chat state is in Postgres.
//!
//! This cache holds the last CHAT_RECENT_CACHE_MAX messages per (channel_type, channel_id).
//! It is only appended to on successful message persistence.

use std::{collections::HashMap, sync::RwLock};

use shared_types::ChatMessageContainer;
#[cfg(feature = "ssr")]
use shared_types::{
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
};

/// Max messages kept per channel in the recent cache.
pub const CHAT_RECENT_CACHE_MAX: usize = 50;

/// Key for the recent-messages cache: (channel_type, channel_id).
pub type ChatChannelKey = (String, String);

#[cfg(feature = "ssr")]
#[derive(Debug, Default)]
pub(crate) struct ChatCacheSnapshot {
    pub tournament_channels: u64,
    pub tournament_messages: u64,
    pub game_spectator_channels: u64,
    pub game_spectator_messages: u64,
    pub game_player_channels: u64,
    pub game_player_messages: u64,
    pub direct_channels: u64,
    pub direct_messages: u64,
}

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

    #[cfg(feature = "ssr")]
    pub(crate) fn snapshot_counts(&self) -> ChatCacheSnapshot {
        let cache = self
            .recent_cache
            .read()
            .unwrap_or_else(|error| error.into_inner());
        let mut snapshot = ChatCacheSnapshot::default();
        for ((channel_type, _), messages) in cache.iter() {
            let message_count = messages.len() as u64;
            match channel_type.as_str() {
                CHANNEL_TYPE_TOURNAMENT_LOBBY => {
                    snapshot.tournament_channels += 1;
                    snapshot.tournament_messages += message_count;
                }
                CHANNEL_TYPE_GAME_SPECTATORS => {
                    snapshot.game_spectator_channels += 1;
                    snapshot.game_spectator_messages += message_count;
                }
                CHANNEL_TYPE_GAME_PLAYERS => {
                    snapshot.game_player_channels += 1;
                    snapshot.game_player_messages += message_count;
                }
                CHANNEL_TYPE_DIRECT => {
                    snapshot.direct_channels += 1;
                    snapshot.direct_messages += message_count;
                }
                _ => {}
            }
        }
        snapshot
    }

    #[cfg(test)]
    pub(crate) fn recent_len(&self, channel_type: &str, channel_id: &str) -> Option<usize> {
        self.recent_cache
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(&(channel_type.to_string(), channel_id.to_string()))
            .map(Vec::len)
    }
}
