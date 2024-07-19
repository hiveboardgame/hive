use super::stats::PingStats;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct Pings {
    pub pings: HashMap<Uuid, PingStats>,
}

impl Default for Pings {
    fn default() -> Self {
        Self::new()
    }
}

impl Pings {
    pub fn new() -> Self {
        Self {
            pings: HashMap::new(),
        }
    }

    pub fn set_nonce(&mut self, user_id: Uuid, nonce: u64) {
        let ping_stats = self.pings.entry(user_id).or_default();
        ping_stats.set_nonce(nonce);
    }

    pub fn update(&mut self, user_id: Uuid, nonce: u64) -> f64 {
        let ping_stats = self.pings.entry(user_id).or_default();
        ping_stats.update(nonce)
    }

    pub fn value(&self, user_id: Uuid) -> f64 {
        if let Some(ping_stats) = self.pings.get(&user_id) {
            return ping_stats.value();
        }
        0.0
    }
}
