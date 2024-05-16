use serde::{Deserialize, Serialize};
use shared_types::ChallengeDetails;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeAction {
    Accept(String), // The user accepts the challenge identified by the nanoid
    Create(ChallengeDetails),
    Decline(String), // Deletes the direct challenge with nanoid
    Delete(String),  // Deletes the challenge with nanoid
    Get(String),     // Gets one challenge
    GetOwn,          // All of the user's open challenges (public, private, direct)
    GetDirected,     // Challenges directed at you
    GetPublic,       // Get public challenges (minus own)
}
