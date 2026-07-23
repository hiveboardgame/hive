use crate::{Certainty, GameSpeed};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatingResponse {
    pub speed: GameSpeed,
    pub rating: u64,
    pub played: i64,
    pub win: i64,
    pub loss: i64,
    pub draw: i64,
    pub certainty: Certainty,
    pub user_uid: Uuid,
}
