#[derive(Clone, Debug)]
pub struct Stats {
    pub samples: usize,
    pub total: f64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            samples: 0,
            total: 0.0,
        }
    }

    pub fn record(&mut self, value: f64) {
        self.samples += 1;
        self.total += value;
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
