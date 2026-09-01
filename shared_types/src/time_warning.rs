use crate::{FlashStyle, GameSpeed, TimeMode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAX_TIME_WARNINGS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningTrigger {
    Proportional,
    Remaining(u32),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Once,
    EachTurn,
    EverySecond,
}

impl Repeat {
    pub const ALL: [Repeat; 3] = [Repeat::Once, Repeat::EachTurn, Repeat::EverySecond];

    pub fn label(self) -> &'static str {
        match self {
            Repeat::Once => "Once",
            Repeat::EachTurn => "Each turn",
            Repeat::EverySecond => "Every second",
        }
    }

    fn insistence(self) -> u8 {
        match self {
            Repeat::Once => 0,
            Repeat::EachTurn => 1,
            Repeat::EverySecond => 2,
        }
    }

    pub fn strongest(self, other: Repeat) -> Repeat {
        if other.insistence() > self.insistence() {
            other
        } else {
            self
        }
    }
}

fn default_flash_styles() -> Vec<FlashStyle> {
    vec![FlashStyle::Board]
}

fn every_real_time_speed() -> Vec<GameSpeed> {
    GameSpeed::real_time_speeds()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWarning {
    pub at: WarningTrigger,
    pub sound: bool,
    pub flash: bool,
    #[serde(default = "default_flash_styles")]
    pub flash_styles: Vec<FlashStyle>,
    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default = "every_real_time_speed")]
    pub speeds: Vec<GameSpeed>,
}

impl TimeWarning {
    pub fn new(at: WarningTrigger) -> Self {
        Self {
            at,
            sound: true,
            flash: false,
            flash_styles: default_flash_styles(),
            repeat: Repeat::Once,
            speeds: every_real_time_speed(),
        }
    }

    fn is_usable(&self) -> bool {
        match self.at {
            WarningTrigger::Proportional => true,
            WarningTrigger::Remaining(seconds) => seconds > 0,
        }
    }
}

pub fn default_time_warnings() -> Vec<TimeWarning> {
    vec![
        TimeWarning::new(WarningTrigger::Proportional),
        TimeWarning {
            at: WarningTrigger::Remaining(5),
            sound: true,
            flash: true,
            flash_styles: default_flash_styles(),
            repeat: Repeat::Once,
            speeds: every_real_time_speed(),
        },
    ]
}

pub fn trigger_at(
    warning: &TimeWarning,
    time_mode: TimeMode,
    base: Option<Duration>,
    increment: Option<Duration>,
) -> Option<Duration> {
    if time_mode != TimeMode::RealTime {
        return None;
    }
    let base = base?;
    let increment = increment.unwrap_or_default();
    match warning.at {
        WarningTrigger::Proportional => Some(base / 10 + increment * 2),
        WarningTrigger::Remaining(seconds) => {
            let threshold = Duration::from_secs(u64::from(seconds));
            (threshold < base / 2).then_some(threshold)
        }
    }
}

pub fn rearm_margin(warning: &TimeWarning, increment: Option<Duration>) -> Duration {
    let increment = increment.unwrap_or_default();
    match warning.at {
        WarningTrigger::Proportional => (increment * 2).max(Duration::from_secs(1)),
        WarningTrigger::Remaining(_) => Duration::from_secs(2),
    }
}

pub fn sanitize_time_warnings(warnings: Vec<TimeWarning>) -> Vec<TimeWarning> {
    let mut kept: Vec<TimeWarning> = Vec::with_capacity(MAX_TIME_WARNINGS);
    for warning in warnings.into_iter().filter(TimeWarning::is_usable) {
        if kept.iter().any(|existing| existing.at == warning.at) {
            continue;
        }
        kept.push(warning);
        if kept.len() == MAX_TIME_WARNINGS {
            break;
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remaining(seconds: u32) -> TimeWarning {
        TimeWarning::new(WarningTrigger::Remaining(seconds))
    }

    #[test]
    fn caps_the_stage_count() {
        let kept = sanitize_time_warnings(vec![
            remaining(60),
            remaining(30),
            remaining(15),
            remaining(10),
            remaining(5),
        ]);
        assert_eq!(kept.len(), MAX_TIME_WARNINGS);
        assert_eq!(kept[0].at, WarningTrigger::Remaining(60));
        assert_eq!(kept[2].at, WarningTrigger::Remaining(15));
    }

    #[test]
    fn drops_zero_second_and_duplicate_triggers() {
        let kept = sanitize_time_warnings(vec![
            remaining(0),
            remaining(5),
            remaining(5),
            TimeWarning::new(WarningTrigger::Proportional),
        ]);
        assert_eq!(
            kept.iter().map(|w| w.at).collect::<Vec<_>>(),
            vec![WarningTrigger::Remaining(5), WarningTrigger::Proportional]
        );
    }

    #[test]
    fn an_explicitly_empty_list_stays_empty() {
        assert!(sanitize_time_warnings(Vec::new()).is_empty());
    }

    fn secs(n: u64) -> Option<Duration> {
        Some(Duration::from_secs(n))
    }

    #[test]
    fn proportional_keeps_the_original_formula() {
        let warning = TimeWarning::new(WarningTrigger::Proportional);
        let at = trigger_at(&warning, TimeMode::RealTime, secs(600), secs(5));
        assert_eq!(at, secs(70));
    }

    #[test]
    fn absolute_stages_are_skipped_when_they_would_be_a_metronome() {
        let warning = remaining(10);
        assert_eq!(
            trigger_at(&warning, TimeMode::RealTime, secs(180), None),
            secs(10)
        );
        assert_eq!(
            trigger_at(&warning, TimeMode::RealTime, secs(15), None),
            None
        );
        assert_eq!(
            trigger_at(&warning, TimeMode::RealTime, secs(20), None),
            None
        );
    }

    #[test]
    fn only_real_time_games_warn() {
        let warning = remaining(5);
        assert_eq!(
            trigger_at(&warning, TimeMode::Correspondence, secs(86400), None),
            None
        );
        assert_eq!(trigger_at(&warning, TimeMode::Untimed, None, None), None);
    }

    #[test]
    fn margins_never_reach_zero() {
        let proportional = TimeWarning::new(WarningTrigger::Proportional);
        assert_eq!(rearm_margin(&proportional, None), Duration::from_secs(1));
        assert_eq!(
            rearm_margin(&proportional, secs(5)),
            Duration::from_secs(10)
        );
        assert_eq!(rearm_margin(&remaining(5), secs(5)), Duration::from_secs(2));
    }
}
