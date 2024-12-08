use crate::websocket::lag_tracking::{decaying_stats::DecayingStats, stats::Stats};

#[derive(Clone, Debug)]
pub struct LagTracker {
    pub quota_gain: f64,
    pub quota: f64,
    pub quota_max: f64,
    pub lag_estimator: DecayingStats,
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
            lag_estimator: DecayingStats::empty(),
            uncomp_stats: Stats::default(),
            lag_stats: Stats::default(),
            comp_est_sq_err: 0.0,
            comp_est_overs: 0.0,
            comp_estimate: None,
        }
    }

    fn quota_base_inc(base: usize, inc: usize) -> f64 {
        let game_time = base as f64 + inc as f64 * 40.0;
        ((game_time / 2.5 + 15.0) / 1000.0).min(100.0)
    }

    pub fn on_move(&mut self, lag: f64) -> f64 {
        let comp = lag.min(self.quota);
        let uncomped = lag - comp;
        let ce_diff = self.comp_estimate.unwrap_or(1.0) - comp;
        let new_quota = (self.quota + self.quota_gain - comp).min(self.quota_max);

        if uncomped != 0.0 || self.uncomp_stats.samples != 0 {
            self.uncomp_stats.record(uncomped);
        }

        self.lag_stats.record(lag.min(2000.0));
        self.comp_est_sq_err += ce_diff * ce_diff;
        self.comp_est_overs += ce_diff.min(0.0);
        self.quota = new_quota;
        comp
    }

    pub fn record_lag(&mut self, lag: f64) {
        self.lag_estimator.record(lag);
        self.comp_estimate = Some(
            0_f64
                .min(self.lag_estimator.mean - 0.8 * self.lag_estimator.deviation)
                .min(self.quota_max),
        );
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
