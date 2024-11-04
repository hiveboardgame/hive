use crate::{common::GameActionResponse, providers::tournament_ready::TournamentReadySignal};
use leptos::*;
use leptos_use::{use_interval_fn, utils::Pausable};

pub fn handle_ready(gar: GameActionResponse) {
    let ready = expect_context::<TournamentReadySignal>().signal;
    ready.update(|r| {
        r.entry(gar.game_id.clone())
            .or_default()
            .insert(gar.user_id);
    });
    let Pausable { .. } = use_interval_fn(
        move || {
            ready.update(|r| {
                r.remove(&gar.game_id);
            })
        },
        30000,
    );
}
