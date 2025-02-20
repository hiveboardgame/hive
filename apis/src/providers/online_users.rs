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
