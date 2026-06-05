use super::snapshot::retain_snapshot_btree_map;
use crate::{common::UserStatus, responses::UserResponse};
use leptos::prelude::*;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Copy)]
pub struct OnlineUsersSignal {
    pub signal: RwSignal<OnlineUsersState>,
    /// Usernames touched by `add`/`remove` since the last `begin_resync`.
    /// `snapshot_apply` consults this so a user who came online or went
    /// offline between snapshot collection and snapshot delivery is never
    /// overwritten by stale snapshot data.
    resync_dirty: StoredValue<HashSet<String>>,
}

impl Default for OnlineUsersSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl OnlineUsersSignal {
    pub fn new() -> Self {
        Self {
            signal: RwSignal::new(OnlineUsersState::new()),
            resync_dirty: StoredValue::new(HashSet::new()),
        }
    }

    pub fn begin_resync(&self) {
        self.resync_dirty.update_value(|d| d.clear());
    }

    pub fn remove(&mut self, username: String) {
        self.resync_dirty.update_value(|d| {
            d.insert(username.clone());
        });
        let user_exists = self.signal.with_untracked(|s| {
            s.username_user.contains_key(&username) || s.username_status.contains_key(&username)
        });
        if !user_exists {
            return;
        }
        self.signal.update(|s| {
            s.username_user.remove(&username);
            s.username_status.remove(&username);
        });
    }

    pub fn add(&mut self, user_response: UserResponse, status: UserStatus) {
        let username = user_response.username.clone();
        self.resync_dirty.update_value(|d| {
            d.insert(username.clone());
        });
        let should_update = self.signal.with_untracked(|s| {
            s.username_user.get(&username) != Some(&user_response)
                || s.username_status.get(&username) != Some(&status)
        });
        if !should_update {
            return;
        }
        self.signal.update(|s| {
            s.username_user.insert(username.clone(), user_response);
            s.username_status.insert(username, status);
        })
    }

    pub fn snapshot_apply(&mut self, users: Vec<UserResponse>) {
        let dirty: HashSet<String> = self.resync_dirty.with_value(|d| d.clone());
        let snapshot_names: HashSet<String> = users.iter().map(|u| u.username.clone()).collect();
        self.signal.update(|s| {
            retain_snapshot_btree_map(&mut s.username_user, &snapshot_names, &dirty);
            retain_snapshot_btree_map(&mut s.username_status, &snapshot_names, &dirty);
            for user in users {
                if dirty.contains(&user.username) {
                    continue;
                }
                s.username_user.insert(user.username.clone(), user.clone());
                s.username_status.insert(user.username, UserStatus::Online);
            }
        });
        self.resync_dirty.update_value(|d| d.clear());
    }
}

#[derive(Clone, Debug)]
pub struct OnlineUsersState {
    pub username_user: BTreeMap<String, UserResponse>,
    pub username_status: BTreeMap<String, UserStatus>,
}

impl OnlineUsersState {
    pub fn new() -> Self {
        Self {
            username_user: BTreeMap::new(),
            username_status: BTreeMap::new(),
        }
    }
}

impl Default for OnlineUsersState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_users() {
    provide_context(OnlineUsersSignal::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Takeback;
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
        }
    }

    /// `snapshot_apply` is the load-bearing primitive behind tab-resume stale-state
    /// recovery — if it ever regresses to merge-style semantics, users who went
    /// offline during tab suspension will linger in the local roster forever.
    #[test]
    fn snapshot_apply_drops_users_not_in_new_set() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.add(make_user("alice"), UserStatus::Online);
            signal.add(make_user("bob"), UserStatus::Online);
            // Resync window begins: only carol remains visible to the server.
            signal.begin_resync();

            signal.snapshot_apply(vec![make_user("carol")]);

            let state = signal.signal.get_untracked();
            assert_eq!(state.username_user.len(), 1);
            assert!(state.username_user.contains_key("carol"));
            assert!(!state.username_user.contains_key("alice"));
            assert!(!state.username_user.contains_key("bob"));
            assert_eq!(state.username_status.len(), 1);
            assert_eq!(
                state.username_status.get("carol"),
                Some(&UserStatus::Online)
            );
        });
    }

    #[test]
    fn snapshot_apply_with_empty_vec_clears_everything() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.add(make_user("alice"), UserStatus::Online);
            signal.add(make_user("bob"), UserStatus::Online);
            signal.begin_resync();

            signal.snapshot_apply(Vec::new());

            let state = signal.signal.get_untracked();
            assert!(state.username_user.is_empty());
            assert!(state.username_status.is_empty());
        });
    }

    #[test]
    fn repeated_snapshot_apply_does_not_accumulate() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.begin_resync();
            signal.snapshot_apply(vec![make_user("alice"), make_user("bob")]);
            signal.begin_resync();
            signal.snapshot_apply(vec![make_user("alice"), make_user("bob")]);

            let state = signal.signal.get_untracked();
            assert_eq!(state.username_user.len(), 2);
        });
    }

    /// User added DURING the resync window must survive a snapshot that omits
    /// them (because the server collected its roster before that user joined).
    #[test]
    fn snapshot_apply_preserves_dirty_added_user() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.begin_resync();
            // Incremental Online for `late` lands BEFORE the snapshot.
            signal.add(make_user("late"), UserStatus::Online);

            // Server snapshot, collected earlier, doesn't know about `late`.
            signal.snapshot_apply(vec![make_user("early")]);

            let state = signal.signal.get_untracked();
            assert!(state.username_user.contains_key("early"));
            assert!(
                state.username_user.contains_key("late"),
                "dirty user must survive stale snapshot"
            );
        });
    }

    /// Conversely: a user whose Offline arrived during the resync window must
    /// stay gone even if the snapshot (collected earlier) still has them.
    #[test]
    fn snapshot_apply_respects_dirty_removed_user() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.add(make_user("alice"), UserStatus::Online);
            signal.begin_resync();
            signal.remove("alice".to_string());

            // Stale snapshot still lists alice — must be ignored.
            signal.snapshot_apply(vec![make_user("alice")]);

            let state = signal.signal.get_untracked();
            assert!(
                !state.username_user.contains_key("alice"),
                "dirty Removed must beat stale snapshot"
            );
        });
    }
}
