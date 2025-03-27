use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

// Implement Ord and PartialOrd to make Certainty directly comparable
// The order is Rankable > Provisional > Clueless (for tournament seeding)
impl PartialOrd for Certainty {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Certainty {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher certainty should come first in sorting (Rankable is "less than" in sort order)
        match (self, other) {
            (Certainty::Rankable, Certainty::Rankable) => Ordering::Equal,
            (Certainty::Rankable, _) => Ordering::Less,
            (Certainty::Provisional, Certainty::Rankable) => Ordering::Greater,
            (Certainty::Provisional, Certainty::Provisional) => Ordering::Equal,
            (Certainty::Provisional, Certainty::Clueless) => Ordering::Less,
            (Certainty::Clueless, Certainty::Clueless) => Ordering::Equal,
            (Certainty::Clueless, _) => Ordering::Greater,
        }
    }
}
