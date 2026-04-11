use shared_types::TournamentId;

use crate::responses::TournamentResponse;

#[derive(Debug, Clone)]
pub enum UserAction {
    Challenge,
    Follow,
    Invite(TournamentId),
    Uninvite(TournamentId),
    Message,
    Unfollow,
    Kick(Box<TournamentResponse>),
}
