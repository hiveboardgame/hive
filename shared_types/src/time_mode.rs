use crate::challenge_error::ChallengeError;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeMode {
    Untimed,
    Correspondence,
    RealTime,
}

impl TimeMode {
    pub fn time_remaining(&self, time_left: Duration) -> String {
        match self {
            TimeMode::Untimed => "".to_owned(),
            TimeMode::Correspondence => {
                let duration = time_left.as_secs();
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;

                if hours < 100 {
                    format!("{:02}h {:02}m", hours, minutes)
                } else {
                    format!("{:03}h {:02}m", hours, minutes)
                }
            }
            TimeMode::RealTime => {
                let duration_seconds = time_left.as_secs();
                let minutes = duration_seconds / 60;
                let seconds = duration_seconds % 60;
                if duration_seconds < 10 {
                    let seconds_f32 = time_left.as_secs_f32();
                    format!("{:.1}", seconds_f32)
                } else if minutes < 100 {
                    format!("{:02}:{:02}", minutes, seconds)
                } else {
                    format!("{:03}:{:02}", minutes, seconds)
                }
            }
        }
    }
}

impl fmt::Display for TimeMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            TimeMode::Correspondence => "Correspondence",
            TimeMode::RealTime => "Real Time",
            TimeMode::Untimed => "Untimed",
        };
        write!(f, "{}", time)
    }
}

impl FromStr for TimeMode {
    type Err = ChallengeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Correspondence" => Ok(TimeMode::Correspondence),
            "Real Time" => Ok(TimeMode::RealTime),
            "Untimed" => Ok(TimeMode::Untimed),
            s => Err(ChallengeError::NotValidTimeMode {
                found: s.to_string(),
            }),
        }
    }
}
