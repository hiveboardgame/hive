use super::stats::PingStats;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct Pings {
    pub pings: RwLock<HashMap<Uuid, PingStats>>,
}

impl Default for Pings {
    fn default() -> Self {
        Self::new()
    }
}

impl Pings {
    pub fn new() -> Self {
        Self {
            pings: HashMap::new().into(),
        }
    }

    pub fn set_nonce(&self, user_id: Uuid, nonce: u64) {
        let mut binding = self.pings.write().unwrap();
        let ping_stats = binding.entry(user_id).or_default();
        ping_stats.set_nonce(nonce);
    }

    pub fn update(&self, user_id: Uuid, nonce: u64) -> f64 {
        let mut binding = self.pings.write().unwrap();
        let ping_stats = binding.entry(user_id).or_default();
        ping_stats.update(nonce)
    }

    pub fn value(&self, user_id: Uuid) -> f64 {
        let binding = self.pings.read().unwrap();
        if let Some(ping_stats) = binding.get(&user_id) {
            return ping_stats.value();
        }
        0.0
    }
}
