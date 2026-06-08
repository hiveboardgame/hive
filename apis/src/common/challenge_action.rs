use serde::{Deserialize, Serialize};
use shared_types::{ChallengeDetails, ChallengeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeAction {
    Accept(ChallengeId), // The user accepts the challenge identified by the nanoid
    Create(ChallengeDetails),
    Delete(ChallengeId),          // Deletes the challenge with nanoid
    DeleteMany(Vec<ChallengeId>), // Deletes multiple challenges by nanoid
}
