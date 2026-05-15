//! In-memory recent-chat counters only. All durable chat state is in Postgres.
//!
//! These counters track up to CHAT_RECENT_CACHE_MAX messages per (channel_type, channel_id).

use std::{collections::HashMap, sync::RwLock};

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
    /// Recent message counts per (channel_type, channel_id), capped at CHAT_RECENT_CACHE_MAX.
    recent_counts: RwLock<HashMap<ChatChannelKey, usize>>,
}

impl Chats {
    pub fn new() -> Self {
        Self {
            recent_counts: RwLock::new(HashMap::new()),
        }
    }

    /// Records one recent message for a channel, keeping at most CHAT_RECENT_CACHE_MAX.
    pub fn push_recent(&self, channel_type: &str, channel_id: &str) {
        let key = (channel_type.to_string(), channel_id.to_string());
        let mut counts = self
            .recent_counts
            .write()
            .unwrap_or_else(|error| error.into_inner());
        let count = counts.entry(key).or_default();
        *count = (*count + 1).min(CHAT_RECENT_CACHE_MAX);
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn snapshot_counts(&self) -> ChatCacheSnapshot {
        let counts = self
            .recent_counts
            .read()
            .unwrap_or_else(|error| error.into_inner());
        let mut snapshot = ChatCacheSnapshot::default();
        for ((channel_type, _), message_count) in counts.iter() {
            let message_count = *message_count as u64;
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
}
