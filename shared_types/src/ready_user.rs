use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ReadyUser {
    pub proposer_id: Uuid,
    pub proposer_username: String,
    pub opponent_id: Uuid,
}