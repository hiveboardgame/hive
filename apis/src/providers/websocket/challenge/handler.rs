use crate::{common::server_result::ChallengeUpdate, providers::challenges::ChallengeStateSignal};

use leptos::*;

pub fn handle_challenge(challenge: ChallengeUpdate) {
    match challenge {
        ChallengeUpdate::Challenges(new_challanges) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.add(new_challanges);
        }
        ChallengeUpdate::Removed(nanoid) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.remove(nanoid);
        }
        ChallengeUpdate::Created(challenge) | ChallengeUpdate::Direct(challenge) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.add(vec![challenge]);
        }
    }
}
