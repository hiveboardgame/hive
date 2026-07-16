use shared_types::GameThread;

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
