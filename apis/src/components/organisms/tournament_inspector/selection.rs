use crate::common::focus_after_render;
use leptos::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TournamentSelection {
    Player(Uuid),
}

impl TournamentSelection {
    pub const fn player(self) -> Uuid {
        match self {
            Self::Player(player) => player,
        }
    }
}

pub(super) fn return_focus_to_player(player: Uuid, mounted: ArcRwSignal<bool>) {
    let selector =
        format!("[data-tournament-player-id=\"{player}\"], [data-arena-player-id=\"{player}\"]");
    focus_after_render(selector, mounted);
}
