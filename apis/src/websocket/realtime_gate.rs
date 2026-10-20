use chrono::{DateTime, Utc};
use db_lib::models::{Challenge, Game};
use hive_lib::GameStatus;
use shared_types::TimeMode;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

const NOT_PAUSED: i64 = 0;

#[derive(Debug, thiserror::Error)]
#[error("New realtime games are paused for maintenance.")]
pub struct RealtimeGamesPaused;

#[derive(Debug, Default)]
pub struct RealtimeGate {
    paused_since_micros: AtomicI64,
    starting: AtomicUsize,
}

#[must_use]
pub struct StartPermit<'a> {
    gate: &'a RealtimeGate,
}

impl Drop for StartPermit<'_> {
    fn drop(&mut self) {
        self.gate.starting.fetch_sub(1, Ordering::SeqCst);
    }
}

impl RealtimeGate {
    pub fn is_paused(&self) -> bool {
        self.paused_since_micros.load(Ordering::SeqCst) != NOT_PAUSED
    }

    pub fn paused_since(&self) -> Option<DateTime<Utc>> {
        match self.paused_since_micros.load(Ordering::SeqCst) {
            NOT_PAUSED => None,
            micros => DateTime::from_timestamp_micros(micros),
        }
    }

    pub fn starting(&self) -> usize {
        self.starting.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) -> bool {
        if paused {
            self.paused_since_micros
                .compare_exchange(
                    NOT_PAUSED,
                    Utc::now().timestamp_micros(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
        } else {
            self.paused_since_micros.swap(NOT_PAUSED, Ordering::SeqCst) != NOT_PAUSED
        }
    }

    pub fn admit(&self, time_mode: TimeMode) -> Result<(), RealtimeGamesPaused> {
        if time_mode == TimeMode::RealTime && self.is_paused() {
            return Err(RealtimeGamesPaused);
        }
        Ok(())
    }

    pub fn admit_challenge(&self, challenge: &Challenge) -> Result<(), RealtimeGamesPaused> {
        match challenge.time_mode.parse::<TimeMode>() {
            Ok(time_mode) => self.admit(time_mode),
            Err(_) => Ok(()),
        }
    }

    pub fn begin_start(
        &self,
        time_mode: TimeMode,
    ) -> Result<Option<StartPermit<'_>>, RealtimeGamesPaused> {
        if time_mode != TimeMode::RealTime {
            return Ok(None);
        }
        // Counting before checking means a concurrent stop either refuses this start or sees it.
        self.starting.fetch_add(1, Ordering::SeqCst);
        let permit = StartPermit { gate: self };
        if self.is_paused() {
            return Err(RealtimeGamesPaused);
        }
        Ok(Some(permit))
    }

    pub fn admit_start(&self, game: &Game) -> Result<Option<StartPermit<'_>>, RealtimeGamesPaused> {
        if game.game_status != GameStatus::NotStarted.to_string() {
            return Ok(None);
        }
        match game.time_mode.parse::<TimeMode>() {
            Ok(time_mode) => self.begin_start(time_mode),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_realtime_is_refused_while_paused() {
        let gate = RealtimeGate::default();
        assert!(gate.admit(TimeMode::RealTime).is_ok());

        assert!(gate.set_paused(true));
        assert!(gate.admit(TimeMode::RealTime).is_err());
        assert!(gate.admit(TimeMode::Correspondence).is_ok());
        assert!(gate.admit(TimeMode::Untimed).is_ok());

        assert!(gate.set_paused(false));
        assert!(gate.admit(TimeMode::RealTime).is_ok());
    }

    #[test]
    fn setting_the_current_value_reports_no_change() {
        let gate = RealtimeGate::default();
        assert!(!gate.set_paused(false));
        assert!(gate.set_paused(true));
        assert!(!gate.set_paused(true));
        assert!(gate.is_paused());
    }

    #[test]
    fn a_repeated_stop_keeps_the_original_start_of_the_pause() {
        let gate = RealtimeGate::default();
        assert_eq!(gate.paused_since(), None);

        gate.set_paused(true);
        let since = gate
            .paused_since()
            .expect("a stopped gate knows when it stopped");
        assert!(since <= Utc::now());
        gate.set_paused(true);
        assert_eq!(gate.paused_since(), Some(since));

        gate.set_paused(false);
        assert_eq!(gate.paused_since(), None);
    }

    #[test]
    fn a_start_in_progress_is_counted_until_it_finishes() {
        let gate = RealtimeGate::default();
        assert!(gate
            .begin_start(TimeMode::Correspondence)
            .expect("correspondence is never refused")
            .is_none());
        assert_eq!(gate.starting(), 0);

        let permit = gate
            .begin_start(TimeMode::RealTime)
            .expect("realtime starts are admitted while games are allowed");
        assert_eq!(gate.starting(), 1);
        gate.set_paused(true);
        assert_eq!(gate.starting(), 1);
        drop(permit);
        assert_eq!(gate.starting(), 0);
    }

    #[test]
    fn a_refused_start_leaves_nothing_counted() {
        let gate = RealtimeGate::default();
        gate.set_paused(true);
        assert!(gate.begin_start(TimeMode::RealTime).is_err());
        assert_eq!(gate.starting(), 0);
    }
}
