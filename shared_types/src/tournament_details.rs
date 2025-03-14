use crate::{ScoringMode, StartMode, Tiebreaker, TimeMode, SeedingMode};
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

impl Default for TournamentDetails {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            scoring: ScoringMode::Game,
            tiebreakers: Vec::new(),
            invitees: Vec::new(),
            seats: 0,
            min_seats: 0,
            rounds: 0,
            invite_only: false,
            mode: String::new(),
            time_mode: TimeMode::RealTime,
            time_base: None,
            time_increment: None,
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual,
            starts_at: None,
            round_duration: None,
            series: None,
            seeding_mode: Some(SeedingMode::Standard),
        }
    }
}
