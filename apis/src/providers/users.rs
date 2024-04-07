use crate::{common::server_result::UserStatus, responses::user::UserResponse};
use leptos::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Copy)]
pub struct UserSignal {
    pub signal: RwSignal<UsersState>,
}

impl Default for UserSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl UserSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(UsersState::new()),
        }
    }

    pub fn remove(&mut self, username: String) {
        self.signal.update(|s| {
            s.username_user.remove(&username);
            s.username_status.remove(&username);
        });
    }

    pub fn update_status(&mut self, user_response: UserResponse, status: UserStatus) {
        self.signal.update(|s| {
            s.username_user
                .insert(user_response.username.clone(), user_response.clone());
            s.username_status.insert(user_response.username, status);
        })
    }

    pub fn update_rating(&mut self, user_response: UserResponse) {
        self.signal.update(|s| {
            s.username_user
                .insert(user_response.username.clone(), user_response.clone());
        })
    }
}

#[derive(Clone, Debug)]
pub struct UsersState {
    pub username_user: BTreeMap<String, UserResponse>,
    pub username_status: HashMap<String, UserStatus>,
}

impl UsersState {
    pub fn new() -> Self {
        Self {
            username_user: BTreeMap::new(),
            username_status: HashMap::new(),
        }
    }

    pub fn online(&self) -> BTreeMap<String, UserResponse> {
        let mut online = self.username_user.clone();
        online.retain(|username, _| {
            self.username_status
                .get(username)
                .is_some_and(|status| *status == UserStatus::Online)
        });
        online
    }
}

impl Default for UsersState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_users() {
    provide_context(UserSignal::new())
}
