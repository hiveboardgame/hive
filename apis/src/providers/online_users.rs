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
