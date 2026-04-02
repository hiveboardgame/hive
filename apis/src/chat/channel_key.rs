use shared_types::{
    canonical_dm_channel_id,
    ChatDestination,
    GameId,
    TournamentId,
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_GLOBAL,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChannelKey {
    pub channel_type: String,
    pub channel_id: String,
}

impl ChannelKey {
    pub fn new(channel_type: impl Into<String>, channel_id: impl Into<String>) -> Self {
        Self {
            channel_type: channel_type.into(),
            channel_id: channel_id.into(),
        }
    }

    pub fn direct(current_user_id: Uuid, other_user_id: Uuid) -> Self {
        Self::new(
            CHANNEL_TYPE_DIRECT,
            canonical_dm_channel_id(current_user_id, other_user_id),
        )
    }

    pub fn tournament(tournament_id: &TournamentId) -> Self {
        Self::new(CHANNEL_TYPE_TOURNAMENT_LOBBY, tournament_id.0.clone())
    }

    pub fn game_players(game_id: &GameId) -> Self {
        Self::new(CHANNEL_TYPE_GAME_PLAYERS, game_id.0.clone())
    }

    pub fn game_spectators(game_id: &GameId) -> Self {
        Self::new(CHANNEL_TYPE_GAME_SPECTATORS, game_id.0.clone())
    }

    pub fn global() -> Self {
        Self::new(CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_GLOBAL)
    }

    pub fn from_destination(
        destination: &ChatDestination,
        current_user_id: Option<Uuid>,
    ) -> Option<Self> {
        match destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                Some(Self::tournament(tournament_id))
            }
            ChatDestination::User((other_user_id, _)) => {
                current_user_id.map(|user_id| Self::direct(user_id, *other_user_id))
            }
            ChatDestination::GamePlayers(game_id, ..) => Some(Self::game_players(game_id)),
            ChatDestination::GameSpectators(game_id, ..) => Some(Self::game_spectators(game_id)),
            ChatDestination::Global => Some(Self::global()),
        }
    }
}
