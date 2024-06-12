use uuid::Uuid;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UserAction {
    Block,
    Challenge,
    Follow,
    Invite(String),
    Message,
    Unblock,
    Unfollow,
}
