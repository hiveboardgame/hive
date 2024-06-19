use shared_types::TournamentId;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UserAction {
    Block,
    Challenge,
    Follow,
    Invite(TournamentId),
    Message,
    Unblock,
    Unfollow,
}
