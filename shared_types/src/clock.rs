use crate::TimeMode;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealtimeClock {
    pub base_seconds: NonZeroU32,
    pub increment_seconds: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorrespondenceClock {
    DaysPerMove { seconds_per_move: NonZeroU32 },
    TotalTimeEach { seconds_each: NonZeroU32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    content = "control",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Clock {
    Realtime(RealtimeClock),
    Correspondence(CorrespondenceClock),
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ClockPartsError {
    #[error(
        "{mode} time control has invalid base/increment values: base={base:?}, increment={increment:?}"
    )]
    Invalid {
        mode: TimeMode,
        base: Option<i32>,
        increment: Option<i32>,
    },
}

impl Clock {
    pub const fn mode(self) -> TimeMode {
        match self {
            Self::Realtime(_) => TimeMode::RealTime,
            Self::Correspondence(_) => TimeMode::Correspondence,
        }
    }

    pub fn from_time_parts(
        mode: TimeMode,
        base: Option<i32>,
        increment: Option<i32>,
    ) -> Result<Option<Self>, ClockPartsError> {
        let invalid = || ClockPartsError::Invalid {
            mode,
            base,
            increment,
        };
        let positive = |value: i32| {
            u32::try_from(value)
                .ok()
                .and_then(NonZeroU32::new)
                .ok_or_else(invalid)
        };

        match (mode, base, increment) {
            (TimeMode::Untimed, None, None) => Ok(None),
            (TimeMode::RealTime, Some(base), Some(increment)) => {
                let base_seconds = positive(base)?;
                let increment_seconds = u32::try_from(increment).map_err(|_| invalid())?;
                Ok(Some(Self::Realtime(RealtimeClock {
                    base_seconds,
                    increment_seconds,
                })))
            }
            (TimeMode::Correspondence, Some(seconds_each), None) => Ok(Some(Self::Correspondence(
                CorrespondenceClock::TotalTimeEach {
                    seconds_each: positive(seconds_each)?,
                },
            ))),
            (TimeMode::Correspondence, None, Some(seconds_per_move)) => Ok(Some(
                Self::Correspondence(CorrespondenceClock::DaysPerMove {
                    seconds_per_move: positive(seconds_per_move)?,
                }),
            )),
            _ => Err(invalid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_time_parts_normalize_to_one_clock_shape() {
        assert_eq!(
            Clock::from_time_parts(TimeMode::Untimed, None, None),
            Ok(None)
        );
        assert_eq!(
            Clock::from_time_parts(TimeMode::RealTime, Some(300), Some(3)),
            Ok(Some(Clock::Realtime(RealtimeClock {
                base_seconds: NonZeroU32::new(300).unwrap(),
                increment_seconds: 3,
            })))
        );
        assert_eq!(
            Clock::from_time_parts(TimeMode::Correspondence, Some(86400), None),
            Ok(Some(Clock::Correspondence(
                CorrespondenceClock::TotalTimeEach {
                    seconds_each: NonZeroU32::new(86400).unwrap(),
                },
            )))
        );
        assert_eq!(
            Clock::from_time_parts(TimeMode::Correspondence, None, Some(86400)),
            Ok(Some(Clock::Correspondence(
                CorrespondenceClock::DaysPerMove {
                    seconds_per_move: NonZeroU32::new(86400).unwrap(),
                },
            )))
        );
    }

    #[test]
    fn malformed_time_parts_fail_instead_of_becoming_untimed() {
        for (mode, base, increment) in [
            (TimeMode::Untimed, Some(1), None),
            (TimeMode::RealTime, None, Some(1)),
            (TimeMode::RealTime, Some(0), Some(1)),
            (TimeMode::RealTime, Some(1), Some(-1)),
            (TimeMode::Correspondence, Some(1), Some(1)),
            (TimeMode::Correspondence, Some(-1), None),
            (TimeMode::Correspondence, None, Some(0)),
        ] {
            assert!(Clock::from_time_parts(mode, base, increment).is_err());
        }
    }
}
