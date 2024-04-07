use crate::{
    common::server_result::{UserRatingUpdate},
    providers::users::UserSignal,
};

use leptos::logging::log;
use leptos::*;

pub fn handle_user_rating(user_update: UserRatingUpdate) {
    let mut users = expect_context::<UserSignal>();
    log!("{:?}", user_update);
    users.update_rating(user_update.user);
}
