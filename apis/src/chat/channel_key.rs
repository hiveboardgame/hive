use shared_types::{
    canonical_dm_channel_id,
    ChannelType,
    ChatDestination,
    GameId,
    TournamentId,
    CHANNEL_TYPE_GLOBAL,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChannelKey {
    pub channel_type: ChannelType,
    pub channel_id: String,
}

impl ChannelKey {
    pub fn new(channel_type: ChannelType, channel_id: impl Into<String>) -> Self {
        Self {
            channel_type,
            channel_id: channel_id.into(),
        }
    }

    pub fn from_raw(channel_type: &str, channel_id: impl Into<String>) -> Option<Self> {
        Some(Self::new(channel_type.parse().ok()?, channel_id))
    }

    pub fn direct(current_user_id: Uuid, other_user_id: Uuid) -> Self {
        Self::new(
            ChannelType::Direct,
            canonical_dm_channel_id(current_user_id, other_user_id),
        )
    }

    pub fn tournament(tournament_id: &TournamentId) -> Self {
        Self::new(ChannelType::TournamentLobby, tournament_id.0.clone())
    }

    pub fn game_players(game_id: &GameId) -> Self {
        Self::new(ChannelType::GamePlayers, game_id.0.clone())
    }

    pub fn game_spectators(game_id: &GameId) -> Self {
        Self::new(ChannelType::GameSpectators, game_id.0.clone())
    }

    pub fn global() -> Self {
        Self::new(ChannelType::Global, CHANNEL_TYPE_GLOBAL)
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

    pub fn from_destination_for_user(
        destination: &ChatDestination,
        current_user_id: Uuid,
    ) -> Self {
        match destination {
            ChatDestination::TournamentLobby(tournament_id) => Self::tournament(tournament_id),
            ChatDestination::User((other_user_id, _)) => {
                Self::direct(current_user_id, *other_user_id)
            }
            ChatDestination::GamePlayers(game_id, ..) => Self::game_players(game_id),
            ChatDestination::GameSpectators(game_id, ..) => Self::game_spectators(game_id),
            ChatDestination::Global => Self::global(),
        }
    }
}
