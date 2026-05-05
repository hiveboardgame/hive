use crate::{
    providers::game_state::View,
    responses::{GameResponse, HeartbeatResponse},
};
use chrono::{DateTime, Utc};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use shared_types::{Conclusion, GameId, TimeMode};
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
            timer.set_timed_out_color(timeout_loser(game));
        });
    }
}

fn timeout_loser(response: &GameResponse) -> Option<Color> {
    match (&response.conclusion, &response.game_status) {
        (Conclusion::Timeout, GameStatus::Finished(GameResult::Winner(color))) => {
            Some(color.opposite_color())
        }
        _ => None,
    }
}

fn history_time_left(
    response: &GameResponse,
    history_turn: Option<usize>,
    color: Color,
) -> Option<Duration> {
    let base = match response.time_mode {
        TimeMode::RealTime => Duration::from_secs(u64::try_from(response.time_base?).ok()?),
        TimeMode::Correspondence => match (response.time_base, response.time_increment) {
            (Some(base), None) => Duration::from_secs(u64::try_from(base).ok()?),
            (None, Some(increment)) => {
                return Some(Duration::from_secs(u64::try_from(increment).ok()?));
            }
            _ => return None,
        },
        TimeMode::Untimed => return None,
    };
    let Some(turn) = history_turn else {
        return Some(base);
    };

    match color {
        Color::White => {
            if turn.is_multiple_of(2) {
                response.recorded_time_left(turn)
            } else {
                response.recorded_time_left(turn - 1)
            }
        }
        Color::Black => {
            if turn.is_multiple_of(2) {
                turn.checked_sub(1)
                    .map(|turn| response.recorded_time_left(turn))
                    .unwrap_or(Some(base))
            } else {
                response.recorded_time_left(turn)
            }
        }
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

    pub fn update_for_view(
        &mut self,
        response: &GameResponse,
        view: &View,
        history_turn: Option<usize>,
    ) {
        let timeout_color = timeout_loser(response);
        if matches!(view, View::History) {
            let is_terminal_timeout =
                timeout_color.is_some() && response.turn.checked_sub(1) == history_turn;
            let display_time = if is_terminal_timeout {
                response.white_time_left.zip(response.black_time_left)
            } else {
                history_time_left(response, history_turn, Color::White).zip(history_time_left(
                    response,
                    history_turn,
                    Color::Black,
                ))
            };
            if let Some((white_time_left, black_time_left)) = display_time {
                self.white_time_left = Some(white_time_left);
                self.black_time_left = Some(black_time_left);
            }
            self.set_timed_out_color(if is_terminal_timeout {
                timeout_color
            } else {
                None
            });
        } else {
            self.white_time_left = response.white_time_left;
            self.black_time_left = response.black_time_left;
            self.set_timed_out_color(timeout_color);
        }
    }

    fn set_timed_out_color(&mut self, color: Option<Color>) {
        self.white_timed_out = color == Some(Color::White);
        self.black_timed_out = color == Some(Color::Black);
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
