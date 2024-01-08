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
            TimeMode::RealTime | TimeMode::Correspondence => {
                let duration = time_left.as_secs();
                let days = duration / 86400;
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let seconds = duration % 60;
                if days > 0 {
                    if days > 1 || hours == 24 {
                        format!("{:1}d", days)
                    } else {
                        format!("{:1}d {:1}h", days, hours % 24)
                    }
                } else if hours > 0 {
                    format!("{:1}h{:1}m", hours, minutes)
                } else if minutes > 0 {
                    format!("{:1}:{:02}", minutes, seconds)
                } else if duration < 10 {
                    let seconds_f32 = time_left.as_secs_f32();
                    format!("{:.1}", seconds_f32)
                } else {
                    format!("{:1}", seconds)
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
