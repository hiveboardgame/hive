use crate::{
    common::server_result::{UserStatus, UserUpdate},
    providers::online_users::OnlineUsersSignal,
};

use leptos::logging::log;
use leptos::*;

pub fn handle_user_status(user_update: UserUpdate) {
    let mut online_users = expect_context::<OnlineUsersSignal>();
    log!("{:?}", user_update);
    match user_update.status {
        UserStatus::Online => online_users.add(
            user_update.user.expect("User is online"),
            UserStatus::Online,
        ),
        UserStatus::Offline => online_users.remove(user_update.username),
        UserStatus::Away => todo!("We need to do away in the frontend"),
    }
}
