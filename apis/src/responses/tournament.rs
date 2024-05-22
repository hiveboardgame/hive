use crate::responses::user::UserResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TournamentAbstractResponse {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TournamentResponse {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<String>,
    pub invitees: Vec<UserResponse>,
    pub players: Vec<UserResponse>,
    pub organizers: Vec<UserResponse>,
    pub seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::Result;
use db_lib::{models::Tournament, DbPool};

impl TournamentAbstractResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self> {
        let tournament = Tournament::from_uuid(id, pool).await?;
        Self::from_model(&tournament)
    }

    pub fn from_model(tournament: &Tournament) -> Result<Self> {
        Ok(Self {
            id: tournament.id,
            nanoid: tournament.nanoid.clone(),
            name: tournament.name.clone(),
        })
    }
}

impl TournamentResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self> {
        let tournament = Tournament::from_uuid(id, pool).await?;
        Self::from_model(&tournament, pool).await
    }

    pub async fn from_model(tournament: &Tournament, pool: &DbPool) -> Result<Self> {
        // TODO: make this one query
        let mut invitees = Vec::new();
        for uuid in tournament.invitees.iter().flatten() {
            invitees.push(UserResponse::from_uuid(uuid, pool).await?);
        }
        let mut players = Vec::new();
        for user in tournament.players(pool).await? {
            players.push(UserResponse::from_model(&user, pool).await?);
        }
        let mut organizers = Vec::new();
        for user in tournament.organizers(pool).await? {
            organizers.push(UserResponse::from_model(&user, pool).await?);
        }
        Ok(Self {
            id: tournament.id,
            nanoid: tournament.nanoid.clone(),
            name: tournament.name.clone(),
            description: tournament.description.clone(),
            scoring: tournament.scoring.clone(), // TODO: make a type for this
            players,
            organizers,
            tiebreaker: tournament.tiebreaker.clone().into_iter().flatten().collect(),
            invitees,
            seats: tournament.seats,
            rounds: tournament.rounds,
            joinable: tournament.joinable,
            invite_only: tournament.invite_only,
            mode: tournament.mode.clone(),
            time_mode: tournament.time_mode.clone(),
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            band_upper: tournament.band_upper,
            band_lower: tournament.band_lower,
            start_at: tournament.start_at,
            created_at: tournament.created_at,
            updated_at: tournament.updated_at,
        })
    }
}
}}
