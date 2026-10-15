use crate::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum GameSpeed {
    Bullet,
    Blitz,
    Rapid,
    Classic,
    Correspondence,
    Untimed,
    Puzzle,
}

impl GameSpeed {
    fn classify_estimated_seconds(total: i64) -> Self {
        if total < 180 {
            Self::Bullet
        } else if total < 480 {
            Self::Blitz
        } else if total < 1500 {
            Self::Rapid
        } else if total <= 18000 {
            Self::Classic
        } else {
            Self::Correspondence
        }
    }
    pub fn all_rated_games() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic, Correspondence]
    }

    pub fn all_rated() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic, Correspondence, Puzzle]
    }

    pub fn all_games() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic, Correspondence, Untimed]
    }

    pub fn all() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![
            Bullet,
            Blitz,
            Rapid,
            Classic,
            Correspondence,
            Puzzle,
            Untimed,
        ]
    }

    pub fn real_time_speeds() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic]
    }

    pub fn is_real_time(&self) -> bool {
        use GameSpeed::*;
        matches!(self, Bullet | Blitz | Rapid | Classic)
    }

    pub fn from_base_increment(base: Option<i32>, increment: Option<i32>) -> GameSpeed {
        let total = i64::from(base.unwrap_or(0)) + 40 * i64::from(increment.unwrap_or(0));
        if total == 0 {
            GameSpeed::Untimed
        } else {
            Self::classify_estimated_seconds(total)
        }
    }
}

impl From<Clock> for GameSpeed {
    fn from(clock: Clock) -> Self {
        match clock {
            Clock::Realtime(clock) => Self::classify_estimated_seconds(
                i64::from(clock.base_seconds.get()) + 40 * i64::from(clock.increment_seconds),
            ),
            Clock::Correspondence(_) => Self::Correspondence,
        }
    }
}

impl fmt::Display for GameSpeed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            GameSpeed::Bullet => "Bullet",
            GameSpeed::Blitz => "Blitz",
            GameSpeed::Rapid => "Rapid",
            GameSpeed::Classic => "Classic",
            GameSpeed::Correspondence => "Correspondence",
            GameSpeed::Untimed => "Untimed",
            GameSpeed::Puzzle => "Puzzle",
        };
        write!(f, "{time}")
    }
}

use thiserror::Error;
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum GameSpeedError {
    #[error("{found} is not a valid GameSpeed")]
    InvalidGameSpeed { found: String },
}

impl std::str::FromStr for GameSpeed {
    type Err = GameSpeedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Bullet" => Ok(GameSpeed::Bullet),
            "Blitz" => Ok(GameSpeed::Blitz),
            "Rapid" => Ok(GameSpeed::Rapid),
            "Classic" => Ok(GameSpeed::Classic),
            "Correspondence" => Ok(GameSpeed::Correspondence),
            "Untimed" => Ok(GameSpeed::Untimed),
            "Puzzle" => Ok(GameSpeed::Untimed),
            s => Err(GameSpeedError::InvalidGameSpeed {
                found: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CorrespondenceClock, RealtimeClock};
    use std::num::NonZeroU32;

    fn realtime(base_seconds: u32, increment_seconds: u32) -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(base_seconds).expect("test base is non-zero"),
            increment_seconds,
        })
    }

    #[test]
    fn realtime_thresholds_are_classified_once() {
        for (seconds, speed) in [
            (179, GameSpeed::Bullet),
            (180, GameSpeed::Blitz),
            (479, GameSpeed::Blitz),
            (480, GameSpeed::Rapid),
            (1499, GameSpeed::Rapid),
            (1500, GameSpeed::Classic),
            (18000, GameSpeed::Classic),
            (18001, GameSpeed::Correspondence),
        ] {
            assert_eq!(GameSpeed::from(realtime(seconds, 0)), speed);
            assert_eq!(
                GameSpeed::from_base_increment(Some(seconds as i32), Some(0)),
                speed
            );
        }
        assert_eq!(GameSpeed::from(realtime(100, 2)), GameSpeed::Blitz);
    }

    #[test]
    fn non_realtime_controls_keep_their_distinct_speeds() {
        assert_eq!(
            GameSpeed::from(Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
                seconds_each: NonZeroU32::new(86400).unwrap(),
            },)),
            GameSpeed::Correspondence
        );
        assert_eq!(
            GameSpeed::from(Clock::Correspondence(CorrespondenceClock::DaysPerMove {
                seconds_per_move: NonZeroU32::new(86400).unwrap(),
            })),
            GameSpeed::Correspondence
        );
        assert_eq!(
            GameSpeed::from_base_increment(None, None),
            GameSpeed::Untimed
        );
    }
}
