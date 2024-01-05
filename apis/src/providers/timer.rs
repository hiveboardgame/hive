use leptos::logging::log;
use leptos::*;
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
        log!("Updating the timer");
        self.signal.update(|timer| {
            timer.finished = game.finished;
            timer.turn = game.turn;
            timer.white_time_left = game.white_time_left;
            timer.black_time_left = game.black_time_left;
            timer.time_increment = Some(Duration::from_secs(game.time_increment.unwrap() as u64));
            timer.time_mode = game.time_mode.to_owned();
        });
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub finished: bool,
    pub turn: usize,
    pub white_time_left: Option<Duration>,
    pub black_time_left: Option<Duration>,
    pub time_increment: Option<Duration>,
    pub time_mode: String,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            finished: true,
            turn: 0,
            white_time_left: None,
            black_time_left: None,
            time_increment: None,
            time_mode: String::from("Untimed"),
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
