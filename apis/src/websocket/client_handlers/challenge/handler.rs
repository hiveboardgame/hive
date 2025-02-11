use crate::{
    common::ChallengeUpdate,
    providers::{challenges::ChallengeStateSignal, AuthContext, NotificationContext},
    responses::ChallengeResponse,
};
use leptos::prelude::*;

fn filter_challenges(challenges: &mut Vec<ChallengeResponse>) {
    let auth_context = expect_context::<AuthContext>();
    let account = move || match auth_context.user.get_untracked() {
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
    let mut challenges = expect_context::<ChallengeStateSignal>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();
    let account = move || match auth_context.user.get_untracked() {
        Some(Ok(Some(account))) => Some(account),
        _ => None,
    };
    match challenge {
        ChallengeUpdate::Challenges(mut new_challanges) => {
            filter_challenges(&mut new_challanges);
            if let Some(account) = account() {
                for challenge in &new_challanges {
                    if let Some(ref opponent) = challenge.opponent {
                        if opponent.uid == account.user.uid {
                            notifications.challenges.update(|challenges| {
                                challenges.insert(challenge.challenge_id.clone());
                            })
                        }
                    }
                }
            }
            challenges.add(new_challanges);
        }
        ChallengeUpdate::Removed(challenge_id) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.remove(challenge_id.clone());
            notifications.challenges.update(|challenges| {
                challenges.remove(&challenge_id);
            })
        }
        ChallengeUpdate::Created(challenge) | ChallengeUpdate::Direct(challenge) => {
            if let Some(account) = account() {
                if let Some(ref opponent) = challenge.opponent {
                    if opponent.uid == account.user.uid {
                        notifications.challenges.update(|challenges| {
                            challenges.insert(challenge.challenge_id.clone());
                        })
                    }
                }
            }
            let mut new_challenges = vec![challenge];
            filter_challenges(&mut new_challenges);
            challenges.add(new_challenges);
        }
    }
}
