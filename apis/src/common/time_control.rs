use std::fmt;
use std::time::Duration;

#[derive(Clone)]
pub enum TimeControl {
    Untimed,
    // max_time_to_move,
    Correspondence(Duration),
    // starting_time, increment_per_move
    RealTime(Duration, Duration),
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            TimeControl::Untimed => "".to_owned(),
            TimeControl::Correspondence(max_time_to_move) => {
                let duration = max_time_to_move.as_secs();
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;

                if hours < 100 {
                    format!("{:02}h {:02}m", hours, minutes)
                } else {
                    format!("{:03}h {:02}m", hours, minutes)
                }
            }
            TimeControl::RealTime(duration, _) => {
                let duration_seconds = duration.as_secs();
                let minutes = duration_seconds / 60;
                let seconds = duration_seconds % 60;
                if duration_seconds < 10 {
                    let seconds_f32 = duration.as_secs_f32();
                    format!("{:.1}", seconds_f32)
                } else if minutes < 100 {
                    format!("{:02}:{:02}", minutes, seconds)
                } else {
                    format!("{:03}:{:02}", minutes, seconds)
                }
            }
        };
        write!(f, "{}", time)
    }
}
