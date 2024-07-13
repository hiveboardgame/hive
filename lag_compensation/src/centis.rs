#[derive(Clone, Debug, PartialEq)]
pub struct Centis(pub f64);

impl Default for Centis {
    fn default() -> Self {
        Centis::new(0.0)
    }
}

impl std::ops::Add for Centis {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Centis(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Centis {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Centis(self.0 - rhs.0)
    }
}

impl Centis {
    pub fn new(value: f64) -> Self {
        Centis(value)
    }

    pub fn at_most(&self, other: &Centis) -> Self {
        Centis(self.0.min(other.0))
    }

    pub fn non_neg(self) -> Self {
        Centis(0.0_f64.max(self.0))
    }

    pub fn centis(&self) -> f64 {
        self.0
    }

    pub fn of_float(value: f64) -> Self {
        Centis(value.abs())
    }
}
