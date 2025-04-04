use crate::{common::GameActionResponse, providers::GameUpdater};
use leptos::prelude::*;

pub fn handle_start(gar: GameActionResponse) {
    let game_updater = expect_context::<GameUpdater>();
    game_updater.response.set(Some(gar.clone()));
}
