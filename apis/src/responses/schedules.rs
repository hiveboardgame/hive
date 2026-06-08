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
use std::collections::{HashMap, HashSet};
impl ScheduleResponse {
    pub async fn from_model(schedule: Schedule, conn: &mut DbConn<'_>) -> Result<Self> {
        let tournament = Tournament::find(schedule.tournament_id, conn).await?;
        let game = Game::find_by_uuid(&schedule.game_id, conn).await?;
        let proposer_username = User::get_username_by_id(&schedule.proposer_id, conn).await?;
        let opponent_username = User::get_username_by_id(&schedule.opponent_id, conn).await?;
        Ok(Self::from_parts(
            schedule,
            tournament.name,
            TournamentId(tournament.nanoid),
            GameId(game.nanoid),
            proposer_username,
            opponent_username,
        ))
    }

    pub async fn from_models_batch(
        schedules: Vec<Schedule>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>> {
        if schedules.is_empty() {
            return Ok(Vec::new());
        }

        let mut tournament_ids = HashSet::new();
        let mut game_ids = HashSet::new();
        let mut user_ids = HashSet::new();
        for schedule in &schedules {
            tournament_ids.insert(schedule.tournament_id);
            game_ids.insert(schedule.game_id);
            user_ids.insert(schedule.proposer_id);
            user_ids.insert(schedule.opponent_id);
        }

        let tournament_ids: Vec<Uuid> = tournament_ids.into_iter().collect();
        let game_ids: Vec<Uuid> = game_ids.into_iter().collect();
        let user_ids: Vec<Uuid> = user_ids.into_iter().collect();

        let tournaments = Tournament::find_by_uuids(&tournament_ids, conn).await?;
        let games = Game::find_by_game_ids(&game_ids, conn).await?;
        let users = User::find_by_uuids(&user_ids, conn).await?;

        let tournaments: HashMap<Uuid, Tournament> = tournaments
            .into_iter()
            .map(|tournament| (tournament.id, tournament))
            .collect();
        let games: HashMap<Uuid, Game> = games.into_iter().map(|game| (game.id, game)).collect();
        let users: HashMap<Uuid, User> = users.into_iter().map(|user| (user.id, user)).collect();

        let mut responses = Vec::with_capacity(schedules.len());
        for schedule in schedules {
            let tournament = tournaments.get(&schedule.tournament_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Tournament {} not found for schedule {}",
                    schedule.tournament_id,
                    schedule.id
                )
            })?;
            let game = games.get(&schedule.game_id).ok_or_else(|| {
                anyhow::anyhow!("Game {} not found for schedule {}", schedule.game_id, schedule.id)
            })?;
            let proposer = users.get(&schedule.proposer_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Proposer {} not found for schedule {}",
                    schedule.proposer_id,
                    schedule.id
                )
            })?;
            let opponent = users.get(&schedule.opponent_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Opponent {} not found for schedule {}",
                    schedule.opponent_id,
                    schedule.id
                )
            })?;

            responses.push(Self::from_parts(
                schedule,
                tournament.name.clone(),
                TournamentId(tournament.nanoid.clone()),
                GameId(game.nanoid.clone()),
                proposer.username.clone(),
                opponent.username.clone(),
            ));
        }

        Ok(responses)
    }

    fn from_parts(
        schedule: Schedule,
        tournament_name: String,
        tournament_id: TournamentId,
        game_id: GameId,
        proposer_username: String,
        opponent_username: String,
    ) -> Self {
        Self {
            id: schedule.id,
            tournament_name,
            tournament_id,
            proposer_id: schedule.proposer_id,
            proposer_username,
            opponent_id: schedule.opponent_id,
            opponent_username,
            game_id,
            start_t: schedule.start_t,
            agreed: schedule.agreed,
            notified: schedule.notified,
        }
    }
}
}}
