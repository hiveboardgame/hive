use crate::pages::profile_view::ProfileGamesContext;
use leptos::{provide_context, RwSignal};

pub fn provide_profile_games() {
    provide_context(ProfileGamesContext {
        unstarted: RwSignal::new(Vec::new()),
        playing: RwSignal::new(Vec::new()),
        finished: RwSignal::new(Vec::new()),
        finished_last_timestamp: RwSignal::new(None),
        finished_last_id: RwSignal::new(None),
        more_finished: RwSignal::new(true),
        user: RwSignal::new(None),
    });
}
