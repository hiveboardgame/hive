use crate::clock_config::ClockConfig;
use crate::decaying_stats::DecayingStats;
use crate::stats::Stats;

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
            quota: quota_gain * 3.0_f64,
            quota_max: quota_gain * 7.0_f64,
            lag_estimator: DecayingStats::empty(),
            uncomp_stats: Stats::new(),
            lag_stats: Stats::new(),
            comp_est_sq_err: 0.0,
            comp_est_overs: 0.0,
            comp_estimate: None,
        }
    }

    fn quota_base_inc(base: usize, inc: usize) -> f64 {
        100.0_f64.min(((base as f64 + inc as f64 * 40.0_f64) * 2.0 / 5.0 + 15.0_f64) / 1000.0_f64)
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

    pub fn record_lag(&mut self, lag: f64) -> &Self {
        let e = self.lag_estimator.record(lag);
        self.comp_estimate = Some((e.mean - 0.8 * e.deviation).min(0.0).min(self.quota_max));
        self
    }

    pub fn moves(&self) -> usize {
        self.lag_stats.samples
    }

    pub fn lag_mean(&self) -> Option<f64> {
        if self.moves() > 0 {
            Some(self.lag_stats.total / self.moves() as f64)
        } else {
            None
        }
    }

    pub fn comp_est_std_err(&self) -> Option<f64> {
        if self.moves() > 2 {
            Some(self.comp_est_sq_err.sqrt() / ((self.moves() - 2) as f64))
        } else {
            None
        }
    }

    pub fn comp_avg(&self) -> Option<f64> {
        if self.moves() > 0 {
            Some(self.total_comp() / self.moves() as f64)
        } else {
            None
        }
    }

    pub fn total_comp(&self) -> f64 {
        self.total_lag() - self.total_uncomped()
    }

    pub fn total_lag(&self) -> f64 {
        self.lag_stats.total
    }

    pub fn total_uncomped(&self) -> f64 {
        self.uncomp_stats.total
    }

    pub fn with_frame_lag(&mut self, frame_lag: f64, clock: ClockConfig) -> &Self {
        let estimated_cpu_lag = 4.0;
        let max_quota_gain_for_clock = LagTracker::max_quota_gain_for(clock);
        let quota_gain = max_quota_gain_for_clock.min(frame_lag + estimated_cpu_lag);
        self.quota_gain = quota_gain;
        self
    }

    fn max_quota_gain_for(clock: ClockConfig) -> f64 {
        let estimate_total_seconds = clock.estimate_total_seconds;
        100.0_f64.min(estimate_total_seconds / 2.5 + 15.0)
    }
}
