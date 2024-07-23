use crate::responses::{GameResponse, HeartbeatResponse};
use chrono::DateTime;
use chrono::Utc;
use leptos::*;
use shared_types::GameId;
use shared_types::TimeMode;
use std::time::Duration;

#[derive(Clone, Debug, Copy)]
pub struct TimerSignal {
    pub signal: RwSignal<Timer>,
    pub low_time: LowTimeWarning,
}

impl Default for TimerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerSignal {
    pub fn new() -> Self {
        let signal = RwSignal::new(Timer::new());
        Self {
            signal,
            low_time: LowTimeWarning::new(signal),
        }
    }

    pub fn update_from_hb(&self, hb: HeartbeatResponse) {
        if hb.game_id == self.signal.get_untracked().game_id {
            self.signal.update(|timer| {
                timer.white_time_left = Some(hb.white_time_left);
                timer.black_time_left = Some(hb.black_time_left);
            });
        }
    }

    pub fn update_from(&self, game: &GameResponse) {
        self.signal.update(|timer| {
            game.game_id.clone_into(&mut timer.game_id);
            timer.finished = game.finished;
            timer.turn = game.turn;
            timer.white_time_left = game.white_time_left;
            timer.black_time_left = game.black_time_left;
            timer.time_increment = game
                .time_increment
                .map(|inc| Duration::from_secs(inc as u64));
            timer.time_mode = game.time_mode.clone();
            timer.last_interaction = game.last_interaction;
            timer.time_base = game.time_base.map(|base| Duration::from_secs(base as u64));
        });
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub game_id: GameId,
    pub finished: bool,
    pub turn: usize,
    pub white_time_left: Option<Duration>,
    pub black_time_left: Option<Duration>,
    pub time_base: Option<Duration>,
    pub time_increment: Option<Duration>,
    pub time_mode: TimeMode,
    pub last_interaction: Option<DateTime<Utc>>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            game_id: GameId(String::new()),
            finished: false,
            turn: 0,
            white_time_left: None,
            black_time_left: None,
            time_base: None,
            time_increment: None,
            time_mode: TimeMode::Untimed,
            last_interaction: None,
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Copy)]
pub struct LowTimeWarning {
    pub issued: RwSignal<bool>,
    pub refresh_at: Signal<Option<Duration>>,
    pub trigger_at: Signal<Option<Duration>>,
}

impl LowTimeWarning {
    pub fn new(timer: RwSignal<Timer>) -> Self {
        let trigger_at = Signal::derive(move || {
            let timer = timer.get();
            if timer.time_mode == TimeMode::RealTime {
                timer.time_base.map(|base| {
                    base / 10 + timer.time_increment.unwrap_or(Duration::from_secs(0)) * 2
                })
            } else {
                None
            }
        });

        let refresh_at = Signal::derive(move || {
            let timer = timer.get();
            if let (Some(trigger_at), Some(time_increment)) = (trigger_at(), timer.time_increment) {
                Some(trigger_at + time_increment * 2)
            } else {
                None
            }
        });
        Self {
            issued: RwSignal::new(false),
            refresh_at,
            trigger_at,
        }
    }
}

pub fn provide_timer() {
    provide_context(TimerSignal::new())
}
