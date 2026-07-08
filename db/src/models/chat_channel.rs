use crate::schema::chat_channels;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use shared_types::GameThread;
use std::fmt;
use uuid::Uuid;

pub const CHAT_CHANNEL_KIND_DIRECT: &str = "direct";
pub const CHAT_CHANNEL_KIND_GAME_PLAYERS: &str = "game_players";
pub const CHAT_CHANNEL_KIND_GAME_SPECTATORS: &str = "game_spectators";
pub const CHAT_CHANNEL_KIND_GLOBAL: &str = "global";
pub const CHAT_CHANNEL_KIND_TOURNAMENT_LOBBY: &str = "tournament_lobby";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatChannelKind {
    Direct,
    Game(GameThread),
    Global,
    TournamentLobby,
}

impl ChatChannelKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Direct => CHAT_CHANNEL_KIND_DIRECT,
            Self::Game(GameThread::Players) => CHAT_CHANNEL_KIND_GAME_PLAYERS,
            Self::Game(GameThread::Spectators) => CHAT_CHANNEL_KIND_GAME_SPECTATORS,
            Self::Global => CHAT_CHANNEL_KIND_GLOBAL,
            Self::TournamentLobby => CHAT_CHANNEL_KIND_TOURNAMENT_LOBBY,
        }
    }
}

impl fmt::Display for ChatChannelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = chat_channels)]
pub struct ChatChannel {
    pub id: i64,
    pub kind: String,
    pub lookup_key: String,
    pub direct_user_low_id: Option<Uuid>,
    pub direct_user_high_id: Option<Uuid>,
    pub game_id: Option<Uuid>,
    pub tournament_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = chat_channels)]
pub struct NewChatChannel<'a> {
    pub kind: &'a str,
    pub lookup_key: &'a str,
    pub direct_user_low_id: Option<Uuid>,
    pub direct_user_high_id: Option<Uuid>,
    pub game_id: Option<Uuid>,
    pub tournament_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
