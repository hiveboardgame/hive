use serde::{Deserialize, Serialize};

pub const RANKABLE_DEVIATION: f64 = 95.0;
pub const PROVISIONAL_DEVIATION: f64 = 120.0;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Certainty {
    Clueless,
    Provisional,
    Rankable,
}

impl Certainty {
    pub fn from_deviation(deviation: f64) -> Self {
        if deviation < RANKABLE_DEVIATION {
            return Certainty::Rankable;
        }
        if deviation < PROVISIONAL_DEVIATION {
            return Certainty::Provisional;
        }
        Certainty::Clueless
    }
}
