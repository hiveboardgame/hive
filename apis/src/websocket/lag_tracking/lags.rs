use crate::websocket::lag_tracking::lag_tracker::LagTracker;
use shared_types::GameId;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct Lags {
    trackers: RwLock<HashMap<(Uuid, GameId), LagTracker>>,
}

#[derive(Debug)]
pub struct PendingLag {
    key: (Uuid, GameId),
    pub seconds: f64,
    tracker: LagTracker,
}

impl Lags {
    pub fn new() -> Self {
        Self {
            trackers: RwLock::new(HashMap::new()),
        }
    }

    // The budget only moves once the move is accepted, so a rejected attempt can neither spend
    // nor refill it.
    pub fn preview(
        &self,
        uuid: Uuid,
        game: GameId,
        lag_ms: f64,
        base: usize,
        inc: usize,
    ) -> PendingLag {
        let mut tracker = self
            .trackers
            .read()
            .ok()
            .and_then(|trackers| trackers.get(&(uuid, game.clone())).cloned())
            .unwrap_or_else(|| LagTracker::new(base, inc));
        let seconds = tracker.on_move(lag_ms / 1000.0);
        PendingLag {
            key: (uuid, game),
            seconds,
            tracker,
        }
    }

    pub fn commit(&self, pending: PendingLag) {
        if let Ok(mut trackers) = self.trackers.write() {
            trackers.insert(pending.key, pending.tracker);
        }
    }

    pub fn remove(&self, uuid: Uuid, game: GameId) {
        if let Ok(mut trackers) = self.trackers.write() {
            trackers.remove(&(uuid, game));
        }
    }

    pub fn remove_pair(&self, uuid_a: Uuid, uuid_b: Uuid, game: GameId) {
        if let Ok(mut trackers) = self.trackers.write() {
            trackers.remove(&(uuid_a, game.clone()));
            trackers.remove(&(uuid_b, game));
        }
    }

    /// Best-effort length used by telemetry snapshots. Returns `None` if the
    /// lock is poisoned so the caller can skip the gauge cleanly.
    pub fn snapshot_len(&self) -> Option<usize> {
        self.trackers.read().ok().map(|t| t.len())
    }
}

impl Default for Lags {
    fn default() -> Self {
        Self::new()
    }
}
