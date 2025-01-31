use crate::responses::UserResponse;
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Copy)]
pub struct UserSearchSignal {
    pub signal: RwSignal<BTreeMap<String, UserResponse>>,
}

impl Default for UserSearchSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl UserSearchSignal {
    pub fn new() -> Self {
        Self {
            signal: RwSignal::new(BTreeMap::new()),
        }
    }

    pub fn set(&mut self, search_results: Vec<UserResponse>) {
        self.signal.update(|s| {
            s.clear();
            for user in search_results {
                s.insert(user.username.clone(), user);
            }
        })
    }
}

pub fn provide_user_search() {
    provide_context(UserSearchSignal::new())
}
