#[derive(Clone, Debug)]
pub struct DecayingStats {
    pub mean: f64,
    pub deviation: f64,
    pub decay: f64,
}

impl DecayingStats {
    pub fn first(value: f64) -> Self {
        DecayingStats {
            mean: value,
            deviation: 0.04,
            decay: 0.85,
        }
    }

    pub fn record(&mut self, value: f64) {
        let delta = self.mean - value;
        self.mean = value + self.decay * delta;
        self.deviation = self.decay * self.deviation + (1.0 - self.decay) * delta.abs();
    }
}
