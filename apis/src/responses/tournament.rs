use super::{GameResponse, UserResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{Standings, Tiebreaker, TimeMode, TournamentId, TournamentStatus};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentAbstractResponse {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentResponse {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub standings: Standings,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreakers: Vec<Tiebreaker>,
    pub invitees: Vec<UserResponse>,
    pub players: HashMap<Uuid, UserResponse>,
    pub organizers: Vec<UserResponse>,
    pub games: Vec<GameResponse>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub status: TournamentStatus,
    pub start_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::Result;
use db_lib::{models::Tournament, DbConn};
use shared_types::TournamentGameResult;
use std::str::FromStr;

impl TournamentAbstractResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        let tournament = Tournament::from_uuid(id, conn).await?;
        Self::from_model(&tournament)
    }

    pub fn from_model(tournament: &Tournament) -> Result<Self> {
        Ok(Self {
            id: tournament.id,
            tournament_id: TournamentId(tournament.nanoid.clone()),
            name: tournament.name.clone(),
        })
    }
}

impl TournamentResponse {
    pub async fn from_tournament_id(
        tournament_id: &TournamentId,
        conn: &mut DbConn<'_>,
    ) -> Result<Box<Self>> {
        let tournament = Tournament::find_by_tournament_id(tournament_id, conn).await?;
        Self::from_model(&tournament, conn).await
    }

    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Box<Self>> {
        let tournament = Tournament::from_uuid(id, conn).await?;
        Self::from_model(&tournament, conn).await
    }

    pub async fn from_model(tournament: &Tournament, conn: &mut DbConn<'_>) -> Result<Box<Self>> {
        // TODO: make this one query
        let mut invitees = Vec::new();
        for user in tournament.invitees(conn).await? {
            invitees.push(UserResponse::from_model(&user, conn).await?);
        }
        let mut players = HashMap::new();
        for user in tournament.players(conn).await? {
            players.insert(user.id, UserResponse::from_model(&user, conn).await?);
        }
        let mut organizers = Vec::new();
        for user in tournament.organizers(conn).await? {
            organizers.push(UserResponse::from_model(&user, conn).await?);
        }
        let games = tournament.games(conn).await?;
        let mut game_responses = Vec::new();
        let mut standings = Standings::new();
        for game in games {
            standings.add_result(
                game.white_id,
                game.black_id,
                game.white_rating.unwrap_or(0.0),
                game.black_rating.unwrap_or(0.0),
                TournamentGameResult::from_str(&game.tournament_game_result)?,
            );
            game_responses.push(GameResponse::from_model(&game, conn).await?);
        }
        standings.enforce_tiebreakers();
        Ok(Box::new(Self {
            id: tournament.id,
            tournament_id: TournamentId(tournament.nanoid.clone()),
            name: tournament.name.clone(),
            description: tournament.description.clone(),
            standings,
            scoring: tournament.scoring.clone(), // TODO: make a type for this
            players,
            organizers,
            games: game_responses,
            tiebreakers: tournament
                .tiebreaker
                .clone()
                .into_iter().flatten().flat_map(|t| Tiebreaker::from_str(&t).ok()).collect(),
            invitees,
            seats: tournament.seats,
            min_seats: tournament.min_seats,
            rounds: tournament.rounds,
            joinable: tournament.joinable,
            invite_only: tournament.invite_only,
            mode: tournament.mode.clone(),
            time_mode: TimeMode::from_str(&tournament.time_mode)?,
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            band_upper: tournament.band_upper,
            band_lower: tournament.band_lower,
            status: TournamentStatus::from_str(&tournament.status)?,
            start_at: tournament.start_at,
            started_at: tournament.started_at,
            round_duration: tournament.round_duration,
            created_at: tournament.created_at,
            updated_at: tournament.updated_at,
        }))
    }
}
}}
