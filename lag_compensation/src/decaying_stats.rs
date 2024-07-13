#[derive(Clone, Debug)]
pub struct DecayingStats {
    pub mean: f64,
    pub deviation: f64,
    pub decay: f64,
}

impl DecayingStats {
    pub fn record(&self, value: f64) -> Self {
        let delta = self.mean - value;
        DecayingStats {
            mean: value + self.decay * delta,
            deviation: self.decay * self.deviation + (1.0 - self.decay) * delta.abs(),
            decay: self.decay,
        }
    }

    pub fn empty() -> Self {
        DecayingStats {
            mean: 0.0,
            deviation: 4.0,
            decay: 0.85,
        }
    }
}
