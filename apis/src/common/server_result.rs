use super::{client_message::SubscriptionAttempt, game_reaction::GameReaction};
use crate::responses::{
    ChallengeResponse,
    GameResponse,
    HeartbeatResponse,
    ScheduleResponse,
    TournamentPatch,
    UserResponse,
};
use serde::{Deserialize, Serialize};
use shared_types::{ChallengeId, ChatMessageContainer, ConversationKey, GameId, TournamentId};
use std::{fmt, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerResult {
    Ok(Box<ServerMessage>),
    Err(ExternalServerError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalServerError {
    Unauthorized {
        reason: String,
    },
    ChatSend {
        key: ConversationKey,
        client_id: Uuid,
        error: ChatSendError,
    },
    ChatSubscribe {
        attempt: SubscriptionAttempt,
        error: SubscriptionError,
    },
    Request {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChatSendError {
    ClientIdConflict,
    RateLimited,
    DirectRestricted,
    AdminOnly,
    TournamentRestricted,
    PlayersRestricted,
    SpectatorsRestricted,
    Unavailable,
}

impl ChatSendError {
    pub fn reason(&self) -> &str {
        match self {
            Self::ClientIdConflict => {
                "This message retry conflicts with the original delivery. Send it as a new message."
            }
            Self::RateLimited => "Too many messages. Please wait and try again.",
            Self::DirectRestricted => "You cannot send messages to this user.",
            Self::AdminOnly => "Global chat requires an administrator.",
            Self::TournamentRestricted => "Only tournament participants can send messages.",
            Self::PlayersRestricted => "Only players can send messages here.",
            Self::SpectatorsRestricted => {
                "Players cannot send to spectator chat while the game is ongoing."
            }
            Self::Unavailable => "Unable to send the chat message.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubscriptionError {
    AccessDenied,
    Unavailable,
    RateLimited { retry_after: Duration },
}

impl SubscriptionError {
    pub fn reason(&self) -> &str {
        match self {
            Self::AccessDenied => "You cannot read this chat",
            Self::Unavailable => "The chat subscription request failed",
            Self::RateLimited { .. } => "Too many chat subscription attempts",
        }
    }
}

impl fmt::Display for ExternalServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let reason = match self {
            Self::Unauthorized { reason } | Self::Request { reason } => reason,
            Self::ChatSend { error, .. } => error.reason(),
            Self::ChatSubscribe { error, .. } => error.reason(),
        };
        write!(f, "WebSocket request failed: {reason}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Challenge(ChallengeUpdate),
    Chat(ChatMessageContainer),
    ChatRead {
        key: ConversationKey,
        last_read_message_id: i64,
    },
    ChatSubscribed(SubscriptionAttempt),
    ConnectionUpdated(Uuid, String),
    Error(String),
    Game(Box<GameUpdate>),
    // sent to everyone in the game when a user joins the game
    Join(Uuid),
    /// Authoritative lobby state sent on connect and Resync.
    LobbySnapshot(Box<LobbySnapshot>),
    Ping {
        nonce: u64,
        value: f64,
    },
    Schedule(ScheduleUpdate),
    Tournament(TournamentUpdate),
    UserSettings(UserSettingsUpdate),
    UserStatus(UserUpdate),
    RedirectLink(String),
}

/// Authoritative best-effort lobby state sent on connect and Resync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbySnapshot {
    pub tournament_invitations: Vec<TournamentId>,
    pub tournament_organizer_invitations: Vec<TournamentId>,
    pub schedule_notifications: Vec<ScheduleResponse>,
    pub urgent_games: Vec<GameResponse>,
    pub challenges: Vec<ChallengeResponse>,
    pub tv_games: Vec<GameResponse>,
    pub online_users: Vec<UserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TournamentUpdate {
    CatalogChanged(TournamentId),
    Created(TournamentId),
    Declined(TournamentId),
    Deleted(TournamentId),
    Finished(TournamentId),
    Invited(TournamentId),
    OrganizerInvited(TournamentId),
    OrganizerInvitationsClosed(TournamentId),
    OrganizerUninvited(TournamentId),
    OrganizerJoined(TournamentId),
    OrganizerLeft(TournamentId, bool),
    Joined(TournamentId),
    Left(TournamentId),
    Patch {
        tournament_id: TournamentId,
        patch: Box<TournamentPatch>,
    },
    SlotsClosed {
        tournament_id: TournamentId,
        count: u32,
    },
    Started(TournamentId),
    Uninvited(TournamentId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserSettingsUpdate {
    BlockedUser {
        user_id: Uuid,
        blocked: bool,
    },
    TournamentChatMuted {
        tournament_id: TournamentId,
        muted: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameUpdate {
    Reaction(GameActionResponse),
    /// Additive: server-pushed when a game becomes urgent (e.g. opponent moved,
    /// draw offered). Client merges into the local `own` map.
    Urgent(Vec<GameResponse>),
    OwnGameRemoved(GameId),
    Tv(GameResponse),
    Heartbeat(HeartbeatResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActionResponse {
    pub game_action: GameReaction,
    pub game: GameResponse,
    pub game_id: GameId,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeUpdate {
    Created(Box<ChallengeResponse>), // A new challenge was created
    Removed(ChallengeId),            // A challenge was removed
    Direct(Box<ChallengeResponse>),  // Player got directly invited to a game
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdate {
    pub status: UserStatus,
    pub user: Option<UserResponse>,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Offline,
    Away,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleUpdate {
    Changed(Vec<ScheduleResponse>),
    TournamentPurged(TournamentId),
}
