use super::snapshot::apply_snapshot_hash_map;
use crate::responses::ChallengeResponse;
use leptos::prelude::*;
use shared_types::ChallengeId;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Copy)]
pub struct ChallengeStateSignal {
    pub signal: RwSignal<ChallengeState>,
    /// IDs touched by `add_one`/`remove` since the last `begin_resync`. Consulted
    /// by `snapshot_apply` so an incremental update that arrived while the
    /// server was building the snapshot is never overwritten by the older
    /// snapshot data. See the race discussion in `lobby_snapshot.rs`.
    resync_dirty: StoredValue<HashSet<ChallengeId>>,
}

impl Default for ChallengeStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengeStateSignal {
    pub fn new() -> Self {
        Self {
            signal: RwSignal::new(ChallengeState::new()),
            resync_dirty: StoredValue::new(HashSet::new()),
        }
    }

    /// Called when the client is about to send `ClientRequest::Resync` (and on
    /// initial connect). Starts a fresh window for dirty tracking.
    pub fn begin_resync(&self) {
        self.resync_dirty.update_value(|d| d.clear());
    }

    pub fn remove(&mut self, challenger_id: ChallengeId) {
        self.resync_dirty.update_value(|d| {
            d.insert(challenger_id.clone());
        });
        self.signal.update(|s| {
            s.challenges.remove(&challenger_id);
        });
    }

    pub fn add_one(&mut self, challenge: ChallengeResponse) {
        let id = challenge.challenge_id.clone();
        self.resync_dirty.update_value(|d| {
            d.insert(id.clone());
        });
        self.signal.update(|s| {
            s.challenges.insert(id, challenge);
        });
    }

    /// Race-safe replace: applies the authoritative server snapshot while
    /// preserving any IDs touched by incremental updates during the resync
    /// window. Truth table:
    ///
    /// | local present | in snapshot | dirty | result          |
    /// |---------------|-------------|-------|-----------------|
    /// | yes           | yes         | no    | overwrite       |
    /// | yes           | yes         | yes   | keep local      |
    /// | yes           | no          | no    | remove          |
    /// | yes           | no          | yes   | keep local      |
    /// | no            | yes         | no    | insert          |
    /// | no            | yes         | yes   | skip (Removed)  |
    pub fn snapshot_apply(&mut self, challenges: Vec<ChallengeResponse>) {
        let dirty: HashSet<ChallengeId> = self.resync_dirty.with_value(|d| d.clone());
        let snapshot_ids: HashSet<ChallengeId> =
            challenges.iter().map(|c| c.challenge_id.clone()).collect();
        self.signal.update(|s| {
            apply_snapshot_hash_map(
                &mut s.challenges,
                &snapshot_ids,
                &dirty,
                challenges,
                |challenge| challenge.challenge_id.clone(),
            );
        });
        self.resync_dirty.update_value(|d| d.clear());
    }
}

#[derive(Clone, Debug)]
pub struct ChallengeState {
    pub challenges: HashMap<ChallengeId, ChallengeResponse>,
}

impl ChallengeState {
    pub fn new() -> Self {
        Self {
            challenges: HashMap::new(),
        }
    }
}

impl Default for ChallengeState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_challenges() {
    provide_context(ChallengeStateSignal::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::responses::{ChallengeResponse, UserResponse};
    use chrono::Utc;
    use hive_lib::ColorChoice;
    use shared_types::{ChallengeVisibility, GameSpeed, Takeback, TimeMode};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_user(name: &str) -> UserResponse {
        UserResponse {
            username: name.to_string(),
            uid: Uuid::new_v4(),
            patreon: false,
            bot: false,
            admin: false,
            deleted: false,
            ratings: HashMap::new(),
            takeback: Takeback::default(),
            lang: None,
        }
    }

    fn make_challenge(id: &str, challenger: &UserResponse) -> ChallengeResponse {
        ChallengeResponse {
            id: Uuid::new_v4(),
            challenge_id: ChallengeId(id.to_string()),
            challenger: challenger.clone(),
            opponent: None,
            game_type: "Base".to_string(),
            rated: false,
            visibility: ChallengeVisibility::Public,
            color_choice: ColorChoice::Random,
            created_at: Utc::now(),
            challenger_rating: 1500,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            speed: GameSpeed::Untimed,
            band_upper: None,
            band_lower: None,
        }
    }

    #[test]
    fn snapshot_apply_drops_ids_not_in_snapshot() {
        let owner = Owner::new();
        owner.with(|| {
            let alice = make_user("alice");
            let mut signal = ChallengeStateSignal::new();
            signal.add_one(make_challenge("a", &alice));
            signal.add_one(make_challenge("b", &alice));
            signal.begin_resync();

            signal.snapshot_apply(vec![make_challenge("c", &alice)]);

            let state = signal.signal.get_untracked();
            assert_eq!(state.challenges.len(), 1);
            assert!(state.challenges.contains_key(&ChallengeId("c".to_string())));
            assert!(!state.challenges.contains_key(&ChallengeId("a".to_string())));
            assert!(!state.challenges.contains_key(&ChallengeId("b".to_string())));
        });
    }

    /// Challenge created during the resync window must survive a stale snapshot.
    #[test]
    fn snapshot_apply_preserves_dirty_added_challenge() {
        let owner = Owner::new();
        owner.with(|| {
            let alice = make_user("alice");
            let mut signal = ChallengeStateSignal::new();
            signal.begin_resync();
            // ChallengeUpdate::Created arrives BEFORE the snapshot.
            signal.add_one(make_challenge("late", &alice));

            // Server snapshot, collected earlier, doesn't know about "late".
            signal.snapshot_apply(vec![make_challenge("early", &alice)]);

            let state = signal.signal.get_untracked();
            assert!(
                state
                    .challenges
                    .contains_key(&ChallengeId("late".to_string())),
                "dirty challenge must survive stale snapshot"
            );
            assert!(state
                .challenges
                .contains_key(&ChallengeId("early".to_string())));
        });
    }

    /// Challenge removed during the resync window must stay gone even if the
    /// snapshot (collected earlier) still has it.
    #[test]
    fn snapshot_apply_respects_dirty_removed_challenge() {
        let owner = Owner::new();
        owner.with(|| {
            let alice = make_user("alice");
            let mut signal = ChallengeStateSignal::new();
            signal.add_one(make_challenge("doomed", &alice));
            signal.begin_resync();
            signal.remove(ChallengeId("doomed".to_string()));

            // Stale snapshot still lists "doomed" — must be ignored.
            signal.snapshot_apply(vec![make_challenge("doomed", &alice)]);

            let state = signal.signal.get_untracked();
            assert!(
                !state
                    .challenges
                    .contains_key(&ChallengeId("doomed".to_string())),
                "dirty Removed must beat stale snapshot"
            );
        });
    }

    /// Documents the contract the `Snapshot` handler relies on for pruning
    /// `NotificationContext.challenges`: after `snapshot_apply`, the local
    /// `challenges` map is authoritative — anything missing from it must
    /// also be pruned from the notification set, or the dropdown component
    /// will panic on `.expect("Challenge exists")` when the user opens it.
    ///
    /// This test exercises the worst case: a direct challenge with an
    /// outstanding notification is dropped by the snapshot (its `Removed`
    /// arrived during the resync window). The handler must observe that
    /// the post-apply map no longer contains the ID and prune the
    /// notification entry to match.
    #[test]
    fn snapshot_apply_lets_caller_prune_stale_notifications() {
        use std::collections::HashSet;

        let owner = Owner::new();
        owner.with(|| {
            let alice = make_user("alice");
            let mut signal = ChallengeStateSignal::new();
            let mut notifications: HashSet<ChallengeId> = HashSet::new();

            // Direct challenge with a red-dot notification.
            let direct = make_challenge("direct-1", &alice);
            notifications.insert(direct.challenge_id.clone());
            signal.add_one(direct);

            // Resync window opens; a Removed for the direct challenge lands
            // before the snapshot. The snapshot — collected earlier on the
            // server — still lists the challenge.
            signal.begin_resync();
            signal.remove(ChallengeId("direct-1".to_string()));
            signal.snapshot_apply(vec![make_challenge("direct-1", &alice)]);

            // The handler now prunes notifications against the post-apply
            // map. The stale notification must go.
            signal.signal.with_untracked(|state| {
                notifications.retain(|id| state.challenges.contains_key(id))
            });

            assert!(
                !notifications.contains(&ChallengeId("direct-1".to_string())),
                "stale direct-challenge notification must be pruned after snapshot_apply"
            );
        });
    }

    #[test]
    fn begin_resync_resets_window() {
        let owner = Owner::new();
        owner.with(|| {
            let alice = make_user("alice");
            let mut signal = ChallengeStateSignal::new();
            signal.begin_resync();
            signal.add_one(make_challenge("transient", &alice));
            // First snapshot apply consumes the dirty mark.
            signal.snapshot_apply(vec![make_challenge("transient", &alice)]);

            // A second resync window starts clean. If the server now omits the
            // challenge, the stale local entry should be cleared — `transient`
            // is no longer dirty.
            signal.begin_resync();
            signal.snapshot_apply(Vec::new());

            let state = signal.signal.get_untracked();
            assert!(state.challenges.is_empty());
        });
    }
}
