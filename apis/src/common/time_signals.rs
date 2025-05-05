use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use shared_types::{CorrespondenceMode, TimeMode};

#[derive(Debug, Clone, Store, Serialize, Deserialize)]
pub struct TimeParams {
    pub time_mode: TimeMode,
    pub corr_mode: CorrespondenceMode,
    pub corr_days: i32,
    pub step_sec: i32,
    pub step_min: i32,
}

impl Default for TimeParams {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeParams {
    pub fn total_seconds(&self) -> i32 {
        let step = self.step_min;
        (match step {
            1..=20 => step,
            21 => step + 4,
            22 => step + 8,
            23..=32 => (step - 20) * 15,
            i32::MIN..=0_i32 | 33_i32..=i32::MAX => unreachable!(),
        }) * 60
    }
    pub fn sec_per_move(&self) -> i32 {
        let step = self.step_sec;
        match step {
            0..=20 => step,
            21 => step + 4,
            22 => step + 8,
            23..=32 => (step - 20) * 15,
            i32::MIN..=-1_i32 | 33_i32..=i32::MAX => unreachable!(),
        }
    }
    pub fn base(&self) -> Option<i32> {
        match self.time_mode {
            TimeMode::Untimed => None,

            TimeMode::RealTime => Some(self.total_seconds()),

            TimeMode::Correspondence => match self.corr_mode {
                CorrespondenceMode::DaysPerMove => None,

                CorrespondenceMode::TotalTimeEach => Some(self.corr_days * 86400),
            },
        }
    }
    pub fn increment(&self) -> Option<i32> {
        match self.time_mode {
            TimeMode::Untimed => None,

            TimeMode::RealTime => Some(self.sec_per_move()),

            TimeMode::Correspondence => match self.corr_mode {
                CorrespondenceMode::DaysPerMove => Some(self.corr_days * 86400),

                CorrespondenceMode::TotalTimeEach => None,
            },
        }
    }
    pub fn new() -> Self {
        let time_mode = TimeMode::RealTime;
        let corr_mode = CorrespondenceMode::DaysPerMove;
        let corr_days = 2_i32;
        let step_min = 10_i32;
        let step_sec = 10_i32;
        Self {
            time_mode,
            corr_mode,
            corr_days,
            step_sec,
            step_min,
        }
    }
}
