use crate::websocket::lag_tracking::{decaying_stats::DecayingStats, stats::Stats};

#[derive(Clone, Debug)]
pub struct LagTracker {
    pub quota_gain: f64,
    pub quota: f64,
    pub quota_max: f64,
    pub lag_estimator: Option<DecayingStats>,
    pub uncomp_stats: Stats,
    pub lag_stats: Stats,
    pub comp_est_sq_err: f64,
    pub comp_est_overs: f64,
    pub comp_estimate: Option<f64>,
}

impl LagTracker {
    pub fn new(base: usize, inc: usize) -> Self {
        let quota_gain = Self::quota_base_inc(base, inc);

        Self {
            quota_gain,
            quota: quota_gain * 3.0,
            quota_max: quota_gain * 7.0,
            lag_estimator: None,
            uncomp_stats: Stats::default(),
            lag_stats: Stats::default(),
            comp_est_sq_err: 0.0,
            comp_est_overs: 0.0,
            comp_estimate: None,
        }
    }

    fn quota_base_inc(base: usize, inc: usize) -> f64 {
        let centis = ((base + inc * 40) * 2 / 5 + 15).min(100);
        centis as f64 / 100.0
    }

    pub fn compensation_for(&self, lag: f64) -> f64 {
        lag.min(self.quota).max(0.0)
    }

    // `comp` may have been granted against an earlier view of the quota that another accepted move
    // has spent from since, so the quota bottoms out at zero instead of going negative.
    pub fn settle(&mut self, lag: f64, comp: f64) {
        let uncomped = lag - comp;
        let ce_diff = self.comp_estimate.unwrap_or(0.01) - comp;
        let new_quota = (self.quota + self.quota_gain - comp).clamp(0.0, self.quota_max);

        if uncomped != 0.0 || self.uncomp_stats.samples != 0 {
            self.uncomp_stats.record(uncomped);
        }

        self.lag_stats.record(lag.min(20.0));
        self.comp_est_sq_err += ce_diff * ce_diff;
        self.comp_est_overs += ce_diff.max(0.0);
        self.quota = new_quota;
        self.record_lag(lag);
    }

    fn record_lag(&mut self, lag: f64) {
        let lag = lag.min(self.quota_max);
        let estimator = match self.lag_estimator.take() {
            Some(mut estimator) => {
                estimator.record(lag);
                estimator
            }
            None => DecayingStats::first(lag),
        };
        self.comp_estimate = Some(
            (estimator.mean - 0.8 * estimator.deviation)
                .max(0.0)
                .min(self.quota),
        );
        self.lag_estimator = Some(estimator);
    }

    // These are currently unused but it would be nice to create a page where you can view all of
    // them at a later date
    // pub fn moves(&self) -> usize {
    //     self.lag_stats.samples
    // }
    //
    // pub fn lag_mean(&self) -> Option<f64> {
    //     if self.moves() > 0 {
    //         Some(self.lag_stats.total / self.moves() as f64)
    //     } else {
    //         None
    //     }
    // }
    //
    // pub fn comp_est_std_err(&self) -> Option<f64> {
    //     if self.moves() > 2 {
    //         Some(self.comp_est_sq_err.sqrt() / ((self.moves() - 2) as f64))
    //     } else {
    //         None
    //     }
    // }
    //
    // pub fn comp_avg(&self) -> Option<f64> {
    //     if self.moves() > 0 {
    //         Some(self.total_comp() / self.moves() as f64)
    //     } else {
    //         None
    //     }
    // }
    //
    // pub fn total_comp(&self) -> f64 {
    //     self.total_lag() - self.total_uncomped()
    // }
    //
    // pub fn total_lag(&self) -> f64 {
    //     self.lag_stats.total
    // }
    //
    // pub fn total_uncomped(&self) -> f64 {
    //     self.uncomp_stats.total
    // }
}

#[cfg(test)]
mod tests {
    use super::LagTracker;

    fn on_move(tracker: &mut LagTracker, lag: f64) -> f64 {
        let comp = tracker.compensation_for(lag);
        tracker.settle(lag, comp);
        comp
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{actual} is not {expected}"
        );
    }

    #[test]
    fn quota_gain_matches_lichess_in_seconds() {
        for (base, inc, gain) in [
            (60, 0, 0.39),
            (180, 2, 1.0),
            (300, 3, 1.0),
            (900, 10, 1.0),
            (15, 0, 0.21),
            (61, 0, 0.39),
        ] {
            let tracker = LagTracker::new(base, inc);
            assert_close(tracker.quota_gain, gain);
            assert_close(tracker.quota, 3.0 * gain);
            assert_close(tracker.quota_max, 7.0 * gain);
        }
    }

    #[test]
    fn compensation_is_bounded_by_the_quota_and_refills_by_the_gain() {
        let mut tracker = LagTracker::new(60, 0);

        assert_close(on_move(&mut tracker, 0.1), 0.1);
        assert_close(tracker.quota, 1.17 + 0.39 - 0.1);
        assert_close(on_move(&mut tracker, 5.0), 1.46);
        assert_close(tracker.quota, 0.39);
        assert_close(on_move(&mut tracker, 5.0), 0.39);
        assert_close(tracker.quota, 0.39);
    }

    #[test]
    fn the_quota_never_grows_past_its_maximum() {
        let mut tracker = LagTracker::new(180, 2);
        for _ in 0..20 {
            on_move(&mut tracker, 0.0);
        }

        assert_close(tracker.quota, tracker.quota_max);
    }

    #[test]
    fn the_compensation_estimate_stays_between_zero_and_the_quota() {
        let mut tracker = LagTracker::new(60, 0);
        on_move(&mut tracker, 0.0);
        assert_eq!(tracker.comp_estimate, Some(0.0));

        for _ in 0..10 {
            on_move(&mut tracker, 5.0);
        }
        let estimate = tracker.comp_estimate.expect("recorded");
        assert!(estimate > 0.0 && estimate <= tracker.quota, "{estimate}");
    }
}
