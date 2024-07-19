use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};

// This value might need to get tweaked a little.
const SLIDING_WINDOW_LENGTH: usize = 3;

#[derive(Debug)]
pub struct PingStats {
    nonces_timestamps: HashMap<u64, DateTime<Utc>>,
    pings: VecDeque<f64>,
}

impl PingStats {
    pub fn new() -> Self {
        Self {
            nonces_timestamps: HashMap::new(),
            pings: VecDeque::new(),
        }
    }
    pub fn value(&self) -> f64 {
        if self.pings.is_empty() {
            return 0.0;
        }
        self.pings.iter().sum::<f64>() / self.pings.len() as f64
    }

    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonces_timestamps.insert(nonce, Utc::now());
    }

    pub fn update(&mut self, nonce: u64) -> f64 {
        if let Some(then) = self.nonces_timestamps.remove(&nonce) {
            if self.pings.len() == SLIDING_WINDOW_LENGTH {
                self.pings.pop_front();
            }
            let ping = Utc::now().signed_duration_since(then).num_milliseconds() as f64 / 2.0;
            self.pings.push_back(ping);
        }
        self.value()
    }
}

impl Default for PingStats {
    fn default() -> Self {
        Self::new()
    }
}
