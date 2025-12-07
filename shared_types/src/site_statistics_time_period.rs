use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(
    Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Copy, Hash, PartialOrd, Ord, Default,
)]
pub enum TimePeriod {
    #[default]
    AllTime,
    LastWeek,
    Last30Days,
    LastYear,
}

impl TimePeriod {
    pub fn all() -> Vec<TimePeriod> {
        use TimePeriod::*;
        vec![AllTime, LastWeek, Last30Days, LastYear]
    }

    // Using arbitrary cutoff date for AllTime to simplify queries construction
    // and avoid conditional statements. Can be actual launch date but I don't remember it.
    pub fn cutoff_date(&self) -> DateTime<Utc> {
        match self {
            TimePeriod::AllTime => NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            TimePeriod::LastWeek => Utc::now() - Duration::days(7),
            TimePeriod::Last30Days => Utc::now() - Duration::days(30),
            TimePeriod::LastYear => Utc::now() - Duration::days(365),
        }
    }
}

impl FromStr for TimePeriod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "All Time" => Ok(Self::AllTime),
            "Last Week" => Ok(Self::LastWeek),
            "Last 30 Days" => Ok(Self::Last30Days),
            "Last Year" => Ok(Self::LastYear),
            _ => Err(format!("Invalid time period: {}", s)),
        }
    }
}

impl fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let period = match self {
            TimePeriod::AllTime => "All Time",
            TimePeriod::LastWeek => "Last Week",
            TimePeriod::Last30Days => "Last 30 Days",
            TimePeriod::LastYear => "Last Year",
        };
        write!(f, "{period}")
    }
}
