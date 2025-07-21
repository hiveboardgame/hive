use crate::responses::{GameResponse, HeartbeatResponse};
use chrono::DateTime;
use chrono::Utc;
use hive_lib::Color;
use leptos::prelude::*;
use shared_types::GameId;
use shared_types::TimeMode;
use std::time::Duration;

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
        let signal = RwSignal::new(Timer::new());
        Self { signal }
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
            timer.time_mode = game.time_mode;
            timer.last_interaction = game.last_interaction;
            timer.time_base = game.time_base.map(|base| Duration::from_secs(base as u64));
        });
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub game_id: GameId,
    pub finished: bool,
    pub white_timed_out: bool,
    pub black_timed_out: bool,
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
            white_timed_out: false,
            black_timed_out: false,
            turn: 0,
            white_time_left: None,
            black_time_left: None,
            time_base: None,
            time_increment: None,
            time_mode: TimeMode::Untimed,
            last_interaction: None,
        }
    }
    pub fn time_left(&self, color: Color) -> Duration {
        match color {
            Color::White => self.white_time_left,
            Color::Black => self.black_time_left,
        }
        .unwrap_or_default()
    }

    pub fn warning_trigger(&self) -> Option<Duration> {
        let increment = self.time_increment.unwrap_or_default();
        match self.time_mode {
            TimeMode::RealTime => self.time_base.map(|b| b / 10 + increment * 2),
            _ => None,
        }
    }
    pub fn warning_refresh(&self) -> Option<Duration> {
        self.warning_trigger()
            .and_then(|trigger| self.time_increment.map(|increment| trigger + increment * 2))
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
