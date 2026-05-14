use crate::{
    common::{UserStatus, UserUpdate},
    providers::online_users::OnlineUsersSignal,
    responses::UserResponse,
};

//use leptos::logging::log;
use leptos::prelude::*;

pub fn handle_user_status(user_update: UserUpdate) {
    let mut online_users = expect_context::<OnlineUsersSignal>();
    //log!("{:?}", user_update);
    match user_update.status {
        UserStatus::Online => online_users.add(
            user_update.user.expect("User is online"),
            UserStatus::Online,
        ),
        UserStatus::Offline => online_users.remove(user_update.username),
        UserStatus::Away => todo!("We need to do away in the frontend"),
    }
}

pub fn handle_user_status_batch(users: Vec<UserResponse>) {
    let mut online_users = expect_context::<OnlineUsersSignal>();
    online_users.replace_all(users);
}
