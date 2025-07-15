use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
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

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::Result;
use db_lib::{
    models::{Game, Schedule, Tournament, User},
    DbConn,
};
impl ScheduleResponse {
    pub async fn from_model(schedule: Schedule, conn: &mut DbConn<'_>) -> Result<Self> {
        let tournament = Tournament::find(schedule.tournament_id, conn).await?;
        let game_id = GameId(Game::find_by_uuid(&schedule.game_id, conn).await?.nanoid);
        let proposer_username = User::get_username_by_id(&schedule.proposer_id, conn).await?;
        let opponent_username = User::get_username_by_id(&schedule.opponent_id, conn).await?;
        Ok(Self {
            id: schedule.id,
            tournament_name: tournament.name,
            tournament_id: TournamentId(tournament.nanoid),
            proposer_id: schedule.proposer_id,
            proposer_username,
            opponent_id: schedule.opponent_id,
            opponent_username,
            game_id,
            start_t: schedule.start_t,
            agreed: schedule.agreed,
            notified: schedule.notified,
        })
    }
}
}}
