use crate::functions::users::user_response::UserResponse;
use leptos::{logging::log, *};
//use leptos_router::*;

#[component]
pub fn DisplayProfile(user: UserResponse) -> impl IntoView {
    log!("{:?}", user);
    user.username.into_view()
}

