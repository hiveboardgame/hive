use crate::{
    common::ChallengeUpdate,
    providers::{challenges::ChallengeStateSignal, AuthContext, NotificationContext},
    responses::{AccountResponse, ChallengeResponse},
};
use leptos::prelude::*;
use shared_types::ChallengeId;

fn is_visible_to_account(challenge: &ChallengeResponse, account: &AccountResponse) -> bool {
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
}

fn should_show_challenge(challenge: &ChallengeResponse, account: Option<&AccountResponse>) -> bool {
    account.is_none_or(|account| is_visible_to_account(challenge, account))
}

fn is_directed_to_account(challenge: &ChallengeResponse, account: &AccountResponse) -> bool {
    challenge
        .opponent
        .as_ref()
        .is_some_and(|opponent| opponent.uid == account.user.uid)
}

fn notify_direct_challenges(
    notifications: NotificationContext,
    account: Option<&AccountResponse>,
    challenges: &[ChallengeResponse],
) {
    let Some(account) = account else {
        return;
    };
    notifications.challenges.update(|notification_ids| {
        for challenge in challenges {
            if is_directed_to_account(challenge, account) {
                notification_ids.insert(challenge.challenge_id.clone());
            }
        }
    });
}

pub fn handle_challenge_snapshot(mut new_challenges: Vec<ChallengeResponse>) {
    let mut challenges = expect_context::<ChallengeStateSignal>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();
    let account = auth_context.user;
    account.with(|account| {
        let account = account.as_ref();
        new_challenges.retain(|challenge| should_show_challenge(challenge, account));
        notify_direct_challenges(notifications, account, &new_challenges);
    });
    challenges.snapshot_apply(new_challenges);
    // After applying the snapshot, the challenge map is the authoritative
    // post-resync state. Prune notification IDs whose underlying challenge no
    // longer exists locally so the notification dropdown cannot look up a
    // missing key. This also handles a direct challenge removed during the
    // resync window: `snapshot_apply` keeps the local removal, and the
    // notification follows.
    notifications.challenges.update(|n| {
        challenges
            .signal
            .with_untracked(|state| n.retain(|id| state.challenges.contains_key(id)));
    });
}

fn handle_challenge_removed(challenge_id: ChallengeId) {
    let mut challenges = expect_context::<ChallengeStateSignal>();
    let notifications = expect_context::<NotificationContext>();
    challenges.remove(challenge_id.clone());
    notifications.challenges.update(|challenges| {
        challenges.remove(&challenge_id);
    });
}

fn handle_challenge_added(challenge: ChallengeResponse) {
    let mut challenges = expect_context::<ChallengeStateSignal>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();
    let account = auth_context.user;
    account.with(|account| {
        let account = account.as_ref();
        notify_direct_challenges(notifications, account, std::slice::from_ref(&challenge));
        if should_show_challenge(&challenge, account) {
            challenges.add_one(challenge);
        }
    });
}

pub fn handle_challenge(challenge: ChallengeUpdate) {
    match challenge {
        ChallengeUpdate::Removed(challenge_id) => {
            handle_challenge_removed(challenge_id);
        }
        ChallengeUpdate::Created(challenge) | ChallengeUpdate::Direct(challenge) => {
            handle_challenge_added(challenge);
        }
    }
}
