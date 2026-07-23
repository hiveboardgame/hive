use crate::{GameId, TournamentId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub tournament_name: String,
    pub tournament_id: TournamentId,
    pub proposer_id: Uuid,
    pub proposer_username: String,
    pub opponent_id: Uuid,
    pub opponent_username: String,
    pub game_id: GameId,
    pub start_t: DateTime<Utc>,
    pub agreed: bool,
    pub notified: bool,
}
