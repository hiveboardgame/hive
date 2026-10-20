use crate::responses::GameResponse;
use hive_lib::GameStatus;
use leptos::prelude::*;
use shared_types::TimeMode;

#[derive(Clone, Copy, Debug)]
pub struct RealtimeGames {
    pub paused: RwSignal<bool>,
}

impl RealtimeGames {
    pub fn apply(&self, paused: bool) {
        if self.paused.get_untracked() != paused {
            self.paused.set(paused);
        }
    }

    pub fn blocks(&self, time_mode: TimeMode) -> bool {
        time_mode == TimeMode::RealTime && self.paused.get()
    }

    pub fn blocks_start(&self, game: &GameResponse) -> bool {
        game.game_status == GameStatus::NotStarted && self.blocks(game.time_mode)
    }
}

pub fn provide_realtime_games() {
    provide_context(RealtimeGames {
        paused: RwSignal::new(false),
    });
}
