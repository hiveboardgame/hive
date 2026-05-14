use crate::{common::UserStatus, responses::UserResponse};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Copy)]
pub struct OnlineUsersSignal {
    pub signal: RwSignal<OnlineUsersState>,
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
        }
    }

    pub fn remove(&mut self, username: String) {
        self.signal.update(|s| {
            s.username_user.remove(&username);
            s.username_status.remove(&username);
        });
    }

    pub fn add(&mut self, user_response: UserResponse, status: UserStatus) {
        self.signal.update(|s| {
            s.username_user
                .insert(user_response.username.clone(), user_response.clone());
            s.username_status.insert(user_response.username, status);
        })
    }

    pub fn replace_all(&mut self, users: Vec<UserResponse>) {
        self.signal.update(|s| {
            s.username_user.clear();
            s.username_status.clear();
            for user in users {
                s.username_user.insert(user.username.clone(), user.clone());
                s.username_status
                    .insert(user.username, UserStatus::Online);
            }
        })
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
            ratings: HashMap::new(),
            takeback: Takeback::default(),
        }
    }

    /// `replace_all` is the load-bearing primitive behind tab-resume stale-state
    /// recovery — if it ever regresses to merge-style semantics, users who went
    /// offline during tab suspension will linger in the local roster forever.
    #[test]
    fn replace_all_drops_users_not_in_new_set() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.add(make_user("alice"), UserStatus::Online);
            signal.add(make_user("bob"), UserStatus::Online);

            signal.replace_all(vec![make_user("carol")]);

            let state = signal.signal.get_untracked();
            assert_eq!(state.username_user.len(), 1);
            assert!(state.username_user.contains_key("carol"));
            assert!(!state.username_user.contains_key("alice"));
            assert!(!state.username_user.contains_key("bob"));
            assert_eq!(state.username_status.len(), 1);
            assert_eq!(state.username_status.get("carol"), Some(&UserStatus::Online));
        });
    }

    #[test]
    fn replace_all_with_empty_vec_clears_everything() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.add(make_user("alice"), UserStatus::Online);
            signal.add(make_user("bob"), UserStatus::Online);

            signal.replace_all(Vec::new());

            let state = signal.signal.get_untracked();
            assert!(state.username_user.is_empty());
            assert!(state.username_status.is_empty());
        });
    }

    #[test]
    fn repeated_replace_all_does_not_accumulate() {
        let owner = Owner::new();
        owner.with(|| {
            let mut signal = OnlineUsersSignal::new();
            signal.replace_all(vec![make_user("alice"), make_user("bob")]);
            signal.replace_all(vec![make_user("alice"), make_user("bob")]);

            let state = signal.signal.get_untracked();
            assert_eq!(state.username_user.len(), 2);
        });
    }
}
