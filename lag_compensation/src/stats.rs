#[derive(Clone, Debug)]
pub struct Stats {
    pub samples: usize,
    pub total: f64,
}

impl Stats {
    fn empty() -> Self {
        Stats { samples: 0, total: 0.0 }
    }

    pub fn new() -> Self {
        Self::empty()
    }

    pub fn record(&mut self, value: f64) {
        self.samples += 1;
        self.total += value;
    }
}
