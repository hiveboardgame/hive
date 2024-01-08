use crate::{common::server_result::UserStatus, responses::user::UserResponse};
use leptos::*;
use std::collections::HashMap;

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
            signal: create_rw_signal(OnlineUsersState::new()),
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
    pub username_user: HashMap<String, UserResponse>,
    pub username_status: HashMap<String, UserStatus>,
}

impl OnlineUsersState {
    pub fn new() -> Self {
        Self {
            username_user: HashMap::new(),
            username_status: HashMap::new(),
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
