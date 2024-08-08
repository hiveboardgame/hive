use std::vec;

use crate::pages::profile_view::ProfileGamesContext;
use leptos::{provide_context, RwSignal};

pub fn provide_profile_games() {
    provide_context(ProfileGamesContext {
        unstarted: RwSignal::new(Vec::new()),
        playing: RwSignal::new(Vec::new()),
        finished: RwSignal::new(Vec::new()),
        finished_batch: RwSignal::new(None),
        more_finished: RwSignal::new(true),
        user: RwSignal::new(None),
        speeds: RwSignal::new(vec![]),
    });
}
