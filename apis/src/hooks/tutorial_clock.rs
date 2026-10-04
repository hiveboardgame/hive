use crate::providers::{
    timer::{Timer, TimerSignal},
    tutorial::TutorialContext,
};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use shared_types::TimeMode;
use std::time::Duration;

const TICK: Duration = Duration::from_millis(100);

pub fn use_tutorial_clock(tutorial: TutorialContext, timer: TimerSignal) {
    reset(tutorial, timer);
    Effect::watch(
        move || tutorial.generation.get(),
        move |_, _, _| reset(tutorial, timer),
        false,
    );
    Effect::watch(
        move || {
            tutorial
                .session
                .with(|session| session.as_ref().map(|session| session.state().turn))
        },
        move |progress, _, _| {
            if let Some(turn) = *progress {
                timer.signal.update(|clock| record_turns(clock, turn));
            }
        },
        false,
    );
    use_interval_fn(
        move || {
            if timer.signal.with_untracked(is_running) {
                timer.signal.update(tick);
            }
        },
        TICK.as_millis() as u64,
    );
}

fn reset(tutorial: TutorialContext, timer: TimerSignal) {
    let lesson_clock = tutorial.session.with_untracked(|session| {
        session.as_ref().and_then(|session| {
            session
                .lesson()
                .clock()
                .map(|clock| (clock, session.state().turn))
        })
    });
    timer.signal.set(match lesson_clock {
        Some(((base_minutes, increment_seconds), turn)) => {
            let base = Duration::from_secs(base_minutes * 60);
            Timer {
                turn,
                white_time_left: Some(base),
                black_time_left: Some(base),
                time_base: Some(base),
                time_increment: Some(Duration::from_secs(increment_seconds)),
                time_mode: TimeMode::RealTime,
                ..Timer::new()
            }
        }
        None => Timer::new(),
    });
}

fn is_running(clock: &Timer) -> bool {
    clock.time_mode == TimeMode::RealTime && !clock.finished
}

fn record_turns(clock: &mut Timer, turn: usize) {
    if clock.time_mode != TimeMode::RealTime {
        return;
    }
    let increment = clock.time_increment.unwrap_or_default();
    for moved in clock.turn..turn {
        let mover = if moved.is_multiple_of(2) {
            &mut clock.white_time_left
        } else {
            &mut clock.black_time_left
        };
        *mover = mover.map(|left| left + increment);
    }
    clock.turn = turn;
}

// Flagging sets `finished` in the same update as the zero, so LiveTimer never asks the server about a game that doesn't exist.
fn tick(clock: &mut Timer) {
    let left = if clock.turn.is_multiple_of(2) {
        &mut clock.white_time_left
    } else {
        &mut clock.black_time_left
    };
    let Some(time) = left.as_mut() else {
        return;
    };
    match time.checked_sub(TICK) {
        Some(rest) if !rest.is_zero() => *time = rest,
        _ => {
            *time = Duration::ZERO;
            clock.finished = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn five_plus_four(turn: usize) -> Timer {
        let base = Duration::from_secs(300);
        Timer {
            turn,
            white_time_left: Some(base),
            black_time_left: Some(base),
            time_base: Some(base),
            time_increment: Some(Duration::from_secs(4)),
            time_mode: TimeMode::RealTime,
            ..Timer::new()
        }
    }

    #[test]
    fn each_move_adds_the_increment_to_the_player_who_moved() {
        let mut clock = five_plus_four(4);
        record_turns(&mut clock, 6);
        assert_eq!(clock.white_time_left, Some(Duration::from_secs(304)));
        assert_eq!(clock.black_time_left, Some(Duration::from_secs(304)));
        assert_eq!(clock.turn, 6);
    }

    #[test]
    fn only_the_side_to_move_loses_time() {
        let mut clock = five_plus_four(5);
        tick(&mut clock);
        assert_eq!(clock.white_time_left, Some(Duration::from_secs(300)));
        assert_eq!(clock.black_time_left, Some(Duration::from_millis(299_900)));
    }

    #[test]
    fn running_out_of_time_stops_the_clock_at_zero() {
        let mut clock = five_plus_four(4);
        clock.white_time_left = Some(Duration::from_millis(50));
        tick(&mut clock);
        assert_eq!(clock.white_time_left, Some(Duration::ZERO));
        assert!(clock.finished);
        assert!(!is_running(&clock));
    }
}
