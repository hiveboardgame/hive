use serde::{Deserialize, Serialize};
use shared_types::{ChallengeDetails, ChallengeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeAction {
    Accept(ChallengeId), // The user accepts the challenge identified by the nanoid
    Create(ChallengeDetails),
    Decline(ChallengeId), // Deletes the direct challenge with nanoid
    Delete(ChallengeId),  // Deletes the challenge with nanoid
    Get(ChallengeId),     // Gets one challenge
    GetOwn,               // All of the user's open challenges (public, private, direct)
    GetDirected,          // Challenges directed at you
    GetPublic,            // Get public challenges (minus own)
}
