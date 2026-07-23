use crate::{StartMode, TimeMode, TournamentId, TournamentStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentAbstractResponse {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub name: String,
    pub games_total: usize,
    pub games_played: usize,
    pub players: usize,
    pub player_list: HashSet<Uuid>,
    pub seats: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub status: TournamentStatus,
    pub start_mode: StartMode,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}
