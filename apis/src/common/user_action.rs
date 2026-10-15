use leptos::prelude::*;
use shared_types::TournamentId;

#[derive(Clone)]
pub enum UserAction {
    Block,
    Challenge,
    Follow,
    Invite(TournamentId),
    InviteOrganizer(TournamentId),
    Message,
    Unblock,
    Unfollow,
    /// Select this user (e.g. for search filters). Callback receives Some(username) on select, None on clear.
    Select(Callback<Option<String>>),
}

impl std::fmt::Debug for UserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block => write!(f, "Block"),
            Self::Challenge => write!(f, "Challenge"),
            Self::Follow => write!(f, "Follow"),
            Self::Invite(_) => write!(f, "Invite"),
            Self::InviteOrganizer(_) => write!(f, "InviteOrganizer"),
            Self::Message => write!(f, "Message"),
            Self::Unblock => write!(f, "Unblock"),
            Self::Unfollow => write!(f, "Unfollow"),
            Self::Select(_) => write!(f, "Select"),
        }
    }
}
