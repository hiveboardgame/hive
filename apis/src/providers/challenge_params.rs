use crate::common::TimeSignals;
use leptos::prelude::*;
use shared_types::{CorrespondenceMode, TimeMode};

#[derive(Debug, Clone, Copy)]
pub struct ChallengeParams {
    pub opponent: RwSignal<Option<String>>,
    pub rated: RwSignal<bool>,
    pub with_expansions: RwSignal<bool>,
    pub is_public: RwSignal<bool>,
    pub time_base: Signal<Option<i32>>,
    pub time_increment: Signal<Option<i32>>,
    pub upper_slider: RwSignal<i32>,
    pub lower_slider: RwSignal<i32>,
    pub time_signals: TimeSignals,
}

impl ChallengeParams {
    pub fn new() -> Self {
        let opponent = RwSignal::new(None);
        let upper_slider = RwSignal::new(550);
        let lower_slider = RwSignal::new(-550);
        let time_signals = TimeSignals::default();
        let time_base = Signal::derive(move || match time_signals.time_mode.get() {
            TimeMode::Untimed => None,

            TimeMode::RealTime => Some(time_signals.total_seconds.get()),

            TimeMode::Correspondence => match time_signals.corr_mode.get() {
                CorrespondenceMode::DaysPerMove => None,

                CorrespondenceMode::TotalTimeEach => Some(time_signals.corr_days.get() * 86400),
            },
        });
        let time_increment = Signal::derive(move || match time_signals.time_mode.get() {
            TimeMode::Untimed => None,

            TimeMode::RealTime => Some(time_signals.sec_per_move.get()),

            TimeMode::Correspondence => match time_signals.corr_mode.get() {
                CorrespondenceMode::DaysPerMove => Some(time_signals.corr_days.get() * 86400),

                CorrespondenceMode::TotalTimeEach => None,
            },
        });

        Self {
            opponent,
            rated: RwSignal::new(true),
            with_expansions: RwSignal::new(true),
            is_public: RwSignal::new(true),
            time_base,
            time_increment,
            upper_slider,
            lower_slider,
            time_signals,
        }
    }
}

impl Default for ChallengeParams {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_challenge_params() {
    provide_context(ChallengeParams::default())
}
