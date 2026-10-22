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
    base: usize,
    inc: usize,
}

impl Lags {
    pub fn new() -> Self {
        Self {
            trackers: RwLock::new(HashMap::new()),
        }
    }

    // The budget only moves once the move is accepted, so a rejected attempt can neither spend
    // nor refill it. The commit settles against the budget as it is by then, so two accepted moves
    // that previewed the same budget both pay.
    pub fn preview(
        &self,
        uuid: Uuid,
        game: GameId,
        lag_ms: f64,
        base: usize,
        inc: usize,
    ) -> PendingLag {
        let key = (uuid, game);
        let lag_seconds = lag_ms / 1000.0;
        let seconds = self
            .trackers
            .read()
            .ok()
            .and_then(|trackers| {
                trackers
                    .get(&key)
                    .map(|tracker| tracker.compensation_for(lag_seconds))
            })
            .unwrap_or_else(|| LagTracker::new(base, inc).compensation_for(lag_seconds));
        PendingLag {
            key,
            seconds,
            base,
            inc,
        }
    }

    // A move quicker than its lag gets back only the time it took, and pays only for that.
    pub fn commit(&self, pending: PendingLag, refunded: f64) {
        if let Ok(mut trackers) = self.trackers.write() {
            trackers
                .entry(pending.key)
                .or_insert_with(|| LagTracker::new(pending.base, pending.inc))
                .settle(refunded.min(pending.seconds));
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

#[cfg(test)]
mod tests {
    use super::Lags;
    use shared_types::GameId;
    use uuid::Uuid;

    #[test]
    fn two_accepted_moves_that_previewed_the_same_budget_both_pay() {
        let lags = Lags::default();
        let (user, game) = (Uuid::new_v4(), GameId("shutout".to_string()));
        let preview = || lags.preview(user, game.clone(), 1000.0, 60, 0);

        let first = preview();
        let second = preview();
        assert_eq!((first.seconds, second.seconds), (1.0, 1.0));
        lags.commit(first, 1.0);
        lags.commit(second, 1.0);

        assert_eq!(
            preview().seconds,
            0.0,
            "1.17 s of quota cannot pay for two 1 s refunds"
        );
    }

    #[test]
    fn a_premove_pays_only_for_what_its_clock_got_back() {
        let lags = Lags::default();
        let (user, game) = (Uuid::new_v4(), GameId("premove".to_string()));
        let premove = lags.preview(user, game.clone(), 400.0, 60, 0);
        assert_eq!(premove.seconds, 0.4);

        lags.commit(premove, 0.15);

        let quota = lags.preview(user, game, 5000.0, 60, 0).seconds;
        assert!((quota - (1.17 + 0.39 - 0.15)).abs() < 1e-9, "{quota}");
    }

    #[test]
    fn a_preview_alone_leaves_the_budget_untouched() {
        let lags = Lags::default();
        let (user, game) = (Uuid::new_v4(), GameId("rejected".to_string()));

        for _ in 0..5 {
            assert_eq!(lags.preview(user, game.clone(), 1000.0, 60, 0).seconds, 1.0);
        }
        assert_eq!(lags.snapshot_len(), Some(0));
    }
}
