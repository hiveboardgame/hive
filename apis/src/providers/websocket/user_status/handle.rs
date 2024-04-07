use crate::{
    common::server_result::{UserStatus, UserStatusUpdate},
    providers::users::UserSignal,
};

use leptos::logging::log;
use leptos::*;

pub fn handle_user_status(user_update: UserStatusUpdate) {
    let mut online_users = expect_context::<UserSignal>();
    log!("{:?}", user_update);
    match user_update.status {
        UserStatus::Online => online_users.update_status(
            user_update.user.expect("User is online"),
            UserStatus::Online,
        ),
        UserStatus::Offline => online_users.remove(user_update.username),
        UserStatus::Away => todo!("We need to do away in the frontend"),
    }
}
