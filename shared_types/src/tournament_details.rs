use crate::{ScoringMode, StartMode, Tiebreaker, TimeMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TournamentDetails {
    pub name: String,
    pub description: String,
    pub scoring: ScoringMode,
    pub tiebreakers: Vec<Option<Tiebreaker>>,
    pub invitees: Vec<Option<Uuid>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: StartMode,
    pub starts_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub series: Option<Uuid>,
    pub fully_automated: bool,
    pub third_place_match: bool,
    pub arena_duration_seconds: Option<i32>,
    /// Every value of the point system. `None` means "whatever this mode
    /// normally does", which is what almost every tournament wants.
    pub points: PointSystemDetails,
}

/// A tournament's scoring, in the units organizers think in: 1 for a win, 0.5
/// for a draw. Any value left `None` falls back to the mode's own convention.
/// Values must be multiples of 0.5.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct PointSystemDetails {
    pub win: Option<f64>,
    pub draw: Option<f64>,
    pub loss: Option<f64>,
    /// What a no-show earns. Zero by default; some events score it differently
    /// from a loss over the board.
    pub forfeit_loss: Option<f64>,
    /// A bye a player asked for.
    pub zero_point_bye: Option<f64>,
    /// The bye handed out when the field is odd — a full point in Swiss, and
    /// nothing in a round robin, where everyone gets one.
    pub pairing_allocated_bye: Option<f64>,
}

impl PointSystemDetails {
    /// The engine works in whole numbers, so every value is multiplied by this
    /// on the way in. It has to be at least 2 for a draw to survive.
    pub const SCALE: usize = 2;

    /// The finest step a tournament can ask for, which is exactly the
    /// reciprocal of `SCALE` — anything finer could not be stored. Derived
    /// rather than written out, so the two can never drift apart.
    pub const GRANULARITY: f64 = 1.0 / Self::SCALE as f64;

    pub fn values(&self) -> [Option<f64>; 6] {
        [
            self.win,
            self.draw,
            self.loss,
            self.forfeit_loss,
            self.zero_point_bye,
            self.pairing_allocated_bye,
        ]
    }

    /// Whether every value set is a non-negative multiple of a half point.
    pub fn is_valid(&self) -> bool {
        self.values()
            .into_iter()
            .flatten()
            .all(|value| value >= 0.0 && (value / Self::GRANULARITY).fract().abs() < f64::EPSILON)
    }
}
