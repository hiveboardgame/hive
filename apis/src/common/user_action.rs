use leptos::prelude::*;
use shared_types::TournamentId;

use crate::responses::TournamentResponse;

#[derive(Clone)]
pub enum UserAction {
    Block,
    Challenge,
    Follow,
    Invite(TournamentId),
    Uninvite(TournamentId),
    Message,
    Unblock,
    Unfollow,
    Kick(Box<TournamentResponse>),
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
            Self::Uninvite(_) => write!(f, "Uninvite"),
            Self::Message => write!(f, "Message"),
            Self::Unblock => write!(f, "Unblock"),
            Self::Unfollow => write!(f, "Unfollow"),
            Self::Kick(_) => write!(f, "Kick"),
            Self::Select(_) => write!(f, "Select"),
        }
    }
}
