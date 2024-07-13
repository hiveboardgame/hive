use crate::centis::Centis;
use crate::clock_config::ClockConfig;
use crate::decaying_stats::DecayingStats;
use crate::stats::Stats;

#[derive(Clone, Debug)]
pub struct LagTracker {
    pub quota_gain: Centis,
    pub quota: Centis,
    pub quota_max: Centis,
    pub lag_estimator: DecayingStats,
    pub uncomp_stats: Stats,
    pub lag_stats: Stats,
    pub comp_est_sq_err: f64,
    pub comp_est_overs: Centis,
    pub comp_estimate: Option<Centis>,
}

impl LagTracker {
    pub fn new(base: usize, inc: usize) -> Self {
        let quota_gain = Self::quota_base_inc(base, inc);

        Self {
            quota_gain: Centis(quota_gain),
            quota: Centis(quota_gain * 3.0_f64),
            quota_max: Centis(quota_gain * 7.0_f64),
            lag_estimator: DecayingStats::empty(),
            uncomp_stats: Stats::new(),
            lag_stats: Stats::new(),
            comp_est_sq_err: 0.0,
            comp_est_overs: Centis(0.0),
            comp_estimate: None,
        }
    }

    fn quota_base_inc(base: usize, inc: usize) -> f64 {
        100.0_f64.min(
            ((base as f64 + inc as f64 * 40.0_f64) * 2.0_f64 / 5.0_f64 + 15.0_f64) / 1000.0_f64,
        )
    }

    pub fn on_move(&mut self, lag: Centis) -> Centis {
        let comp = lag.at_most(&self.quota);
        let uncomped = lag.clone() - comp.clone();
        let ce_diff = self.comp_estimate.clone().unwrap_or(Centis::new(1.0)) - comp.clone();
        let new_quota =
            (self.quota.clone() + self.quota_gain.clone() - comp.clone()).at_most(&self.quota_max);

        if uncomped != Centis(0.0) || self.uncomp_stats.samples != 0 {
            self.uncomp_stats.record(uncomped.0);
        }

        self.lag_stats.record((lag.at_most(&Centis(2000.0))).0);

        self.comp_est_sq_err += ce_diff.0 * ce_diff.0;
        self.comp_est_overs = self.comp_est_overs.clone() + ce_diff.non_neg();
        self.quota = new_quota;
        comp
    }

    pub fn record_lag(&mut self, lag: &Centis) -> &Self {
        let e = self.lag_estimator.record(lag.centis());
        self.comp_estimate = Some(
            Centis::of_float(e.mean - 0.8 * e.deviation)
                .non_neg()
                .at_most(&self.quota_max),
        );
        self
    }

    pub fn moves(&self) -> usize {
        self.lag_stats.samples
    }

    pub fn lag_mean(&self) -> Option<Centis> {
        if self.moves() > 0 {
            Some(Centis::of_float(self.lag_stats.total / self.moves() as f64))
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

    pub fn comp_avg(&self) -> Option<Centis> {
        if self.moves() > 0 {
            Some(Centis::of_float(
                self.total_comp().centis() / self.moves() as f64,
            ))
        } else {
            None
        }
    }

    pub fn total_comp(&self) -> Centis {
        Centis::of_float(self.total_lag().centis() - self.total_uncomped().centis())
    }

    pub fn total_lag(&self) -> Centis {
        Centis::of_float(self.lag_stats.total)
    }

    pub fn total_uncomped(&self) -> Centis {
        Centis::of_float(self.uncomp_stats.total)
    }

    pub fn with_frame_lag(&mut self, frame_lag: Centis, clock: ClockConfig) -> &Self {
        let estimated_cpu_lag = Centis::new(4.0); // Assuming a constant value similar to Scala code
        let max_quota_gain_for_clock = LagTracker::max_quota_gain_for(clock);
        let quota_gain = max_quota_gain_for_clock.at_most(&(frame_lag + estimated_cpu_lag));
        self.quota_gain = quota_gain;
        self
    }

    fn max_quota_gain_for(clock: ClockConfig) -> Centis {
        let estimate_total_seconds = clock.estimate_total_seconds;
        Centis::new(100.0_f64.min(estimate_total_seconds * 2.0 / 5.0 + 15.0))
    }
}
