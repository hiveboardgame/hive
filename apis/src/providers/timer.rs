use chrono::DateTime;
use chrono::Utc;
use leptos::*;
use shared_types::time_mode::TimeMode;
use std::time::Duration;

use crate::responses::game::GameResponse;

#[derive(Clone, Debug, Copy)]
pub struct TimerSignal {
    pub signal: RwSignal<Timer>,
}

impl Default for TimerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(Timer::new()),
        }
    }

    pub fn update_from(&self, game: &GameResponse) {
        //log!("Updating the timer");
        self.signal.update(|timer| {
            game.nanoid.clone_into(&mut timer.nanoid);
            timer.finished = game.finished;
            timer.turn = game.turn;
            timer.white_time_left = game.white_time_left;
            timer.black_time_left = game.black_time_left;
            timer.time_increment = game
                .time_increment
                .map(|inc| Duration::from_secs(inc as u64));
            timer.time_mode = game.time_mode.clone();
            timer.last_interaction = game.last_interaction;
        });
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub nanoid: String,
    pub finished: bool,
    pub turn: usize,
    pub white_time_left: Option<Duration>,
    pub black_time_left: Option<Duration>,
    pub time_increment: Option<Duration>,
    pub time_mode: TimeMode,
    pub last_interaction: Option<DateTime<Utc>>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            nanoid: String::new(),
            finished: true,
            turn: 0,
            white_time_left: None,
            black_time_left: None,
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

pub fn provide_timer() {
    provide_context(TimerSignal::new())
}
