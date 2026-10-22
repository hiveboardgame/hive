#[derive(Clone, Debug)]
pub struct LagTracker {
    pub quota_gain: f64,
    pub quota: f64,
    pub quota_max: f64,
}

impl LagTracker {
    pub fn new(base: usize, inc: usize) -> Self {
        let quota_gain = Self::quota_base_inc(base, inc);

        Self {
            quota_gain,
            quota: quota_gain * 3.0,
            quota_max: quota_gain * 7.0,
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
    pub fn settle(&mut self, comp: f64) {
        self.quota = (self.quota + self.quota_gain - comp).clamp(0.0, self.quota_max);
    }
}

#[cfg(test)]
mod tests {
    use super::LagTracker;

    fn on_move(tracker: &mut LagTracker, lag: f64) -> f64 {
        let comp = tracker.compensation_for(lag);
        tracker.settle(comp);
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
}
