use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::tournament_dropouts;

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = tournament_dropouts)]
pub struct TournamentDropout {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    pub dropped_at: DateTime<Utc>,
    pub dropped_by: Uuid,
    pub dropped_in_round: i32,
    pub reason: Option<String>,
}

impl TournamentDropout {
    pub fn new(
        tournament_id: Uuid,
        user_id: Uuid,
        dropped_by: Uuid,
        dropped_in_round: i32,
        reason: Option<String>,
    ) -> Self {
        Self {
            tournament_id,
            user_id,
            dropped_at: Utc::now(),
            dropped_by,
            dropped_in_round,
            reason,
        }
    }
} 