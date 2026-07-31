use super::{GameResponse, UserResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{
    ByeKind,
    ScoringMode,
    Standings,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentId,
    TournamentStatus,
};
use std::collections::{HashMap, HashSet};
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
    /// Arena only. An arena is bounded by a clock rather than a round count, so
    /// this plus `started_at` is what a countdown is derived from — and what
    /// tells a listing whether the arena is still worth joining.
    pub arena_duration_seconds: Option<i32>,
}

/// A round a player sat out, and why — which is what decides what it scored.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ByeResponse {
    pub player: Uuid,
    pub round: i32,
    pub kind: ByeKind,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentResponse {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub standings: Standings,
    pub name: String,
    pub description: String,
    pub scoring: ScoringMode,
    pub tiebreakers: Vec<Tiebreaker>,
    pub invitees: Vec<UserResponse>,
    /// Invitees who said no. Kept separate from `invitees` so an organizer can
    /// tell a decline apart from an invitation still sitting unanswered.
    pub declined_invitees: Vec<UserResponse>,
    pub players: HashMap<Uuid, UserResponse>,
    /// Players who left mid-event. They keep their row and everything they
    /// scored, so without this a leaver is indistinguishable from somebody who
    /// simply has no game in the current round.
    pub withdrawn: HashSet<Uuid>,
    pub organizers: Vec<UserResponse>,
    /// Rounds a player sat out. A bye earns points but produces no game, so it
    /// is invisible in `games` and has to travel separately.
    pub byes: Vec<ByeResponse>,
    pub games: Vec<GameResponse>,
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
    pub status: TournamentStatus,
    pub start_mode: StartMode,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Arena only. With `started_at` this is what the arena countdown is
    /// derived from — an arena ends on its clock, not on a round count.
    pub arena_duration_seconds: Option<i32>,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::Result;
use db_lib::{
    models::{Tournament, TournamentBye},
    DbConn,
};
use std::str::FromStr;

impl TournamentAbstractResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        let tournament = Tournament::from_uuid(id, conn).await?;
        Self::from_model(&tournament, conn).await
    }

    pub async fn from_uuids(ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<HashMap<Uuid, Self>> {
        let tournaments = Tournament::find_by_uuids(ids, conn).await?;
        let mut result = HashMap::new();
        for tournament in tournaments {
            let tournament_response = Self::from_model(&tournament, conn).await?;
            result.insert(tournament.id, tournament_response);
        }
        Ok(result)
    }

    pub async fn from_model(tournament: &Tournament, conn: &mut DbConn<'_>) -> Result<Self> {
        let player_list = tournament.players(conn).await?
        .iter()
        .map(|p| p.id)
        .collect();
        Ok(Self {
            id: tournament.id,
            tournament_id: TournamentId(tournament.nanoid.clone()),
            name: tournament.name.clone(),
            games_total: tournament.number_of_games(conn).await? as usize,
            games_played: tournament.number_of_finished_games(conn).await? as usize,
            players: tournament.number_of_players(conn).await? as usize,
            player_list,
            seats: tournament.seats,
            invite_only: tournament.invite_only,
            mode: tournament.mode.clone(),
            time_mode: TimeMode::from_str(&tournament.time_mode)?,
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            band_upper: tournament.band_upper,
            band_lower: tournament.band_lower,
            status: TournamentStatus::from_str(&tournament.status)?,
            start_mode: StartMode::from_str(&tournament.start_mode)?,
            starts_at: tournament.starts_at,
            ends_at: tournament.ends_at,
            started_at: tournament.started_at,
            updated_at: tournament.updated_at,
            arena_duration_seconds: tournament.arena_duration_seconds,
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
        let mut declined_invitees = Vec::new();
        for user in tournament.declined_invitees(conn).await? {
            declined_invitees.push(UserResponse::from_model(&user, conn).await?);
        }
        let mut players = HashMap::new();
        for user in tournament.players(conn).await? {
            players.insert(user.id, UserResponse::from_model(&user, conn).await?);
        }
        let mut organizers = Vec::new();
        for user in tournament.organizers(conn).await? {
            organizers.push(UserResponse::from_model(&user, conn).await?);
        }
        let withdrawn = tournament
            .withdrawn_players(conn)
            .await?
            .into_iter()
            .collect();
        let byes = TournamentBye::for_tournament(tournament.id, conn)
            .await?
            .into_iter()
            .map(|bye| {
                Ok(ByeResponse {
                    player: bye.user_id,
                    round: bye.round,
                    kind: ByeKind::from_str(&bye.kind)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let games = tournament.games(conn).await?;
        // Standings are the pairing engine's to work out — it replays the whole
        // tournament to produce them, which is the only way the bracket and
        // arena formats can be scored at all.
        let standings = tournament.standings(conn).await?;
        let game_responses = GameResponse::from_games_batch(games, conn).await?;
        Ok(Box::new(Self {
            id: tournament.id,
            tournament_id: TournamentId(tournament.nanoid.clone()),
            name: tournament.name.clone(),
            description: tournament.description.clone(),
            // Taken from the standings rather than the stored column, because
            // the engine prepends the format's primary score — `RawPoints`, or
            // `RoundsSurvived` for a bracket — and drops any stored tiebreaker
            // it does not recognise. These are the columns that actually exist.
            tiebreakers: standings.tiebreakers.clone(),
            standings,
            scoring: ScoringMode::from_str(&tournament.scoring)?,
            players,
            withdrawn,
            organizers,
            byes,
            games: game_responses,
            invitees,
            declined_invitees,
            seats: tournament.seats,
            min_seats: tournament.min_seats,
            rounds: tournament.rounds,
            invite_only: tournament.invite_only,
            mode: tournament.mode.clone(),
            time_mode: TimeMode::from_str(&tournament.time_mode)?,
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            band_upper: tournament.band_upper,
            band_lower: tournament.band_lower,
            status: TournamentStatus::from_str(&tournament.status)?,
            start_mode: StartMode::from_str(&tournament.start_mode)?,
            starts_at: tournament.starts_at,
            ends_at: tournament.ends_at,
            started_at: tournament.started_at,
            round_duration: tournament.round_duration,
            created_at: tournament.created_at,
            updated_at: tournament.updated_at,
            arena_duration_seconds: tournament.arena_duration_seconds,
        }))
    }
}
}}
