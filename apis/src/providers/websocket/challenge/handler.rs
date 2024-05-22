use crate::{
    common::ChallengeUpdate,
    providers::{auth_context::AuthContext, challenges::ChallengeStateSignal},
    responses::ChallengeResponse,
};
use leptos::*;

fn filter_challenges(challenges: &mut Vec<ChallengeResponse>) {
    let auth_context = expect_context::<AuthContext>();
    let account = move || match untrack(auth_context.user) {
        Some(Ok(Some(account))) => Some(account),
        _ => None,
    };
    if let Some(account) = account() {
        challenges.retain(|challenge| {
            if challenge.challenger.uid == account.id {
                return true;
            }
            if let Some(upper) = challenge.band_upper {
                if account.user.rating_for_speed(&challenge.speed) > upper as u64 {
                    return false;
                }
            }
            if let Some(lower) = challenge.band_lower {
                if account.user.rating_for_speed(&challenge.speed) < lower as u64 {
                    return false;
                }
            }
            true
        });
    }
}

pub fn handle_challenge(challenge: ChallengeUpdate) {
    match challenge {
        ChallengeUpdate::Challenges(mut new_challanges) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            filter_challenges(&mut new_challanges);
            challenges.add(new_challanges);
        }
        ChallengeUpdate::Removed(challenger_id) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.remove(challenger_id);
        }
        ChallengeUpdate::Created(challenge) | ChallengeUpdate::Direct(challenge) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            let mut new_challenges = vec![challenge];
            filter_challenges(&mut new_challenges);
            challenges.add(new_challenges);
        }
    }
}
