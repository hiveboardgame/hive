use crate::{providers::user_search::UserSearchSignal, responses::UserResponse};
use leptos::expect_context;

pub fn handle_user_search(results: Vec<UserResponse>) {
    let mut user_search = expect_context::<UserSearchSignal>();
    user_search.set(results);
}
