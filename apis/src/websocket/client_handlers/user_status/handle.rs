use crate::{
    common::{UserStatus, UserUpdate},
    providers::online_users::OnlineUsersSignal,
    responses::UserResponse,
};

use leptos::prelude::*;

pub fn handle_user_status(user_update: UserUpdate) {
    let mut online_users = expect_context::<OnlineUsersSignal>();
    match user_update.status {
        UserStatus::Online => online_users.add(
            user_update.user.expect("User is online"),
            UserStatus::Online,
        ),
        UserStatus::Offline => online_users.remove(user_update.username),
        UserStatus::Away => todo!("We need to do away in the frontend"),
    }
}

/// Applies an authoritative roster snapshot. The signal preserves IDs touched
/// by `add`/`remove` since the matching `begin_resync` so a user who came online
/// after the server began building the snapshot isn't dropped on apply.
pub fn handle_user_status_snapshot(users: Vec<UserResponse>) {
    let mut online_users = expect_context::<OnlineUsersSignal>();
    online_users.snapshot_apply(users);
}
