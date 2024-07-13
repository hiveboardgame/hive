pub struct ClockConfig {
    pub estimate_total_seconds: f64,
}

impl ClockConfig {
    pub fn new(estimate_total_seconds: f64) -> Self {
        ClockConfig { estimate_total_seconds }
    }
}
