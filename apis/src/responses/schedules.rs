use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub proposer_id: Uuid,
    pub opponent_id: Uuid,
    pub game_id: GameId,
    pub start_t: DateTime<Utc>,
    pub agreed: bool,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::Result;
use db_lib::{
    models::{Game, Schedule, Tournament},
    DbConn,
};
impl ScheduleResponse {
    pub async fn from_model(schedule: Schedule, conn: &mut DbConn<'_>) -> Result<Self> {
        let tournament_id =
            TournamentId(Tournament::find(schedule.tournament_id, conn).await?.nanoid);
        let game_id = GameId(Game::find_by_uuid(&schedule.game_id, conn).await?.nanoid);
        Ok(Self {
            id: schedule.id,
            tournament_id,
            proposer_id: schedule.proposer_id,
            opponent_id: schedule.opponent_id,
            game_id,
            start_t: schedule.start_t,
            agreed: schedule.agreed,
        })
    }
}
}}
