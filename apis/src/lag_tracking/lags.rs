use crate::lag_tracking::lag_tracker::LagTracker;
use shared_types::GameId;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct Lags {
    trackers: RwLock<HashMap<(Uuid, GameId), LagTracker>>,
}

impl Lags {
    pub fn new() -> Self {
        Self {
            trackers: RwLock::new(HashMap::new()),
        }
    }

    pub fn track_lag(
        &self,
        uuid: Uuid,
        game: GameId,
        lag: f64,
        base: usize,
        inc: usize,
    ) -> Option<f64> {
        if let Ok(mut uuids_lags) = self.trackers.write() {
            let user_lags = uuids_lags
                .entry((uuid, game))
                .or_insert(LagTracker::new(base, inc));
            user_lags.record_lag(lag / 1000.0);
            let comp = Some(user_lags.on_move(lag / 1000.0));
            return comp;
        }
        None
    }

    pub fn remove(&self, uuid: Uuid, game: GameId) {
        if let Ok(mut trackers) = self.trackers.write() {
            trackers.remove(&(uuid, game));
        }
    }
}

impl Default for Lags {
    fn default() -> Self {
        Self::new()
    }
}
