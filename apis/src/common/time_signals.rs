use leptos::prelude::*;
use shared_types::{CorrespondenceMode, TimeMode};

#[derive(Debug, Copy, Clone)]
pub struct TimeSignals {
    pub time_mode: RwSignal<TimeMode>,
    pub corr_mode: RwSignal<CorrespondenceMode>,
    pub corr_days: RwSignal<i32>,
    pub step_sec: RwSignal<i32>,
    pub step_min: RwSignal<i32>,
    pub total_seconds: Signal<i32>,
    pub sec_per_move: Signal<i32>,
}

impl Default for TimeSignals {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeSignals {
    pub fn new() -> Self {
        let time_mode = RwSignal::new(TimeMode::RealTime);
        let corr_mode = RwSignal::new(CorrespondenceMode::DaysPerMove);
        let corr_days = RwSignal::new(2_i32);
        let step_min = RwSignal::new(10_i32);
        let step_sec = RwSignal::new(10_i32);
        let total_seconds = Signal::derive(move || {
            let step = step_min();
            (match step {
                1..=20 => step,
                21 => step + 4,
                22 => step + 8,
                23..=32 => (step - 20) * 15,
                i32::MIN..=0_i32 | 33_i32..=i32::MAX => unreachable!(),
            }) * 60
        });
        let sec_per_move = Signal::derive(move || {
            let step = step_sec();
            match step {
                0..=20 => step,
                21 => step + 4,
                22 => step + 8,
                23..=32 => (step - 20) * 15,
                i32::MIN..=-1_i32 | 33_i32..=i32::MAX => unreachable!(),
            }
        });
        Self {
            time_mode,
            corr_mode,
            corr_days,
            step_sec,
            step_min,
            total_seconds,
            sec_per_move,
        }
    }
}
