use crate::{common::GameActionResponse, providers::GameUpdater};
use leptos::prelude::*;

pub fn handle_ready(gar: GameActionResponse) {
    let ready = expect_context::<GameUpdater>().tournament_ready;
    ready.set((gar.game_id, gar.user_id));
}
