use crate::{ScoringMode, SeedingMode, StartMode, Tiebreaker, TimeMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub seeding_mode: Option<SeedingMode>,
}
