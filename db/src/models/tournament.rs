#![feature(int_roundings)]
use super::{Game, NewGame, Rating, TournamentInvitation};
use crate::models::tournament;
use crate::{config::DbConfig, get_conn, get_pool};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        ratings,
        tournaments::{
            self, bye, current_round, ends_at, nanoid as nanoid_field, series as series_column,
            started_at, starts_at, status as status_column, updated_at,
        },
        tournaments_organizers, users,
    },
    DbConn,
};
use chrono::{prelude::*, TimeDelta};
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::prelude::*;
use diesel::serialize::{Output, ToSql};
use diesel_async::RunQueryDsl;
use hive_lib::Color;
use itertools::Itertools;
use nanoid::nanoid;
use rand::random;
use serde::{Deserialize, Serialize};
use shared_types::{
    GameSpeed, ScoringMode, SeedingMode, Standings, StartMode, Tiebreaker, TimeMode,
    TournamentDetails, TournamentGameResult, TournamentId, TournamentSortOrder, TournamentStatus,
};
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournaments)]
pub struct NewTournament {
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub bye: Vec<Option<Uuid>>,
    pub current_round: i32,
    pub initial_seeding: Vec<Option<Uuid>>,
    pub seeding_mode: String,
}

impl NewTournament {
    pub fn new(details: TournamentDetails) -> Result<Self, DbError> {
        if matches!(details.time_mode, TimeMode::Untimed) {
            return Err(DbError::InvalidInput {
                info: String::from("How did you trigger this?"),
                error: String::from("Cannot create untimed tournament."),
            });
        }

        // TOOD: @leex add some more validations
        if details.tiebreakers.is_empty() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("No tiebreaker set"),
            });
        }

        if details.time_mode == TimeMode::Correspondence && details.round_duration.is_some() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Cannot set round duration on correspondence tournaments"),
            });
        }

        if details.seats < details.min_seats {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Seats is less than minimun number of seats"),
            });
        }

        if details.rounds < 1 {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Number of rounds needs to be >= 1"),
            });
        }

        if details.rounds > 16 {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Number of rounds needs to <= 16"),
            });
        }

        Ok(Self {
            nanoid: nanoid!(11),
            name: details.name,
            description: details.description,
            scoring: details.scoring.to_string(),
            tiebreaker: details
                .tiebreakers
                .iter()
                .flatten()
                .map(|t| Some(t.to_string()))
                .collect(),
            seats: details.seats,
            min_seats: details.min_seats,
            rounds: details.rounds,
            invite_only: details.invite_only,
            mode: details.mode,
            time_mode: details.time_mode.to_string(),
            time_base: details.time_base,
            time_increment: details.time_increment,
            band_upper: details.band_upper,
            band_lower: details.band_lower,
            start_mode: details.start_mode.to_string(),
            starts_at: details.starts_at,
            ends_at: None,
            started_at: None,
            round_duration: details.round_duration,
            status: TournamentStatus::NotStarted.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: details.series,
            bye: Vec::new(),
            current_round: 0,
            initial_seeding: Vec::new(),
            seeding_mode: details
                .seeding_mode
                .unwrap_or(SeedingMode::Standard)
                .to_string(),
        })
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournaments)]
pub struct Tournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub bye: Vec<Option<Uuid>>, // Vector of (player_id, round_number) tuples for byes
    pub current_round: i32,
    pub initial_seeding: Vec<Option<Uuid>>,
    pub seeding_mode: String,
}

impl Tournament {
    pub async fn create(
        user_id: Uuid,
        new_tournament: &NewTournament,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        // TODO: create only works when user's rating is RANKABLE
        let tournament: Tournament = diesel::insert_into(tournaments::table)
            .values(new_tournament)
            .get_result(conn)
            .await?;
        let tournament_organizer = TournamentOrganizer::new(tournament.id, user_id);
        diesel::insert_into(tournaments_organizers::table)
            .values(tournament_organizer)
            .execute(conn)
            .await?;
        Ok(tournament)
    }

    pub async fn delete(&mut self, user_id: Uuid, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(&user_id, conn).await?;
        diesel::delete(tournaments::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn ensure_not_inivte_only(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        if self.invite_only {
            if self
                .invitees(conn)
                .await?
                .iter()
                .any(|invitee| invitee.id == *user_id)
                || self
                    .organizers(conn)
                    .await?
                    .iter()
                    .any(|organizer| organizer.id == *user_id)
            {
                return Ok(());
            }
            return Err(DbError::TournamentInviteOnly);
        }
        Ok(())
    }

    async fn ensure_not_full(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if self.number_of_players(conn).await? == self.seats as i64 {
            return Err(DbError::TournamentFull);
        }
        Ok(())
    }

    fn ensure_not_started(&self) -> Result<(), DbError> {
        if self.status != TournamentStatus::NotStarted.to_string() {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {}", self.status),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    fn ensure_inprogress(&self) -> Result<(), DbError> {
        if self.status != TournamentStatus::InProgress.to_string() {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {}", self.status),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    pub async fn ensure_games_finished(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if self.number_of_games(conn).await? != self.number_of_finished_games(conn).await? {
            return Err(DbError::InvalidAction {
                info: String::from("Not all games have finished"),
            });
        }
        Ok(())
    }

    pub async fn ensure_user_is_organizer(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let organizers = self.organizers(conn).await?;
        if organizers.iter().any(|o| o.id == *user_id) {
            return Ok(());
        }
        Err(DbError::Unauthorized)
    }

    async fn has_enough_players(&self, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(self.number_of_players(conn).await? >= self.min_seats as i64)
    }

    pub async fn create_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(user_id, conn).await?;
        if TournamentInvitation::exists(&self.id, invitee, conn).await? {
            return Ok(self.clone());
        }
        let invitation = TournamentInvitation::new(self.id, *invitee);
        invitation.insert(conn).await?;
        Ok(self.clone())
    }

    pub async fn finish(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_inprogress()?;
        self.ensure_user_is_organizer(user_id, conn).await?;
        self.ensure_games_finished(conn).await?;
        let tournament = diesel::update(tournaments::table.find(self.id))
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::Finished.to_string()),
            ))
            .get_result(conn)
            .await?;
        Ok(tournament)
    }

    pub async fn retract_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(user_id, conn).await?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, invitee, conn).await {
            invitation.delete(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn decline_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn accept_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_not_full(conn).await?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
            let tournament_user = TournamentUser::new(self.id, *user_id);
            tournament_user.insert(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn add_to_series(
        &self,
        series_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(Some(series_id)))
            .get_result(conn)
            .await?)
    }

    pub async fn remove_from_series(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(None::<Uuid>))
            .get_result(conn)
            .await?)
    }

    pub async fn join(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_not_full(conn).await?;
        self.ensure_not_inivte_only(user_id, conn).await?;
        let players = self.players(conn).await?;
        if players.len() == self.seats as usize {
            return Ok(self.clone());
        }
        if players.iter().any(|player| player.id == *user_id) {
            return Ok(self.clone());
        }
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
        }
        let tournament_user = TournamentUser::new(self.id, *user_id);
        tournament_user.insert(conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn leave(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        TournamentUser::delete(self.id, *user_id, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn kick(
        &self,
        organizer: &Uuid,
        player: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(organizer, conn).await?;
        TournamentUser::delete(self.id, *player, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn from_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table.find(uuid).first(conn).await?)
    }

    pub async fn find_by_uuid(uuid: Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table.find(uuid).first(conn).await?)
    }

    pub async fn from_nanoid(nano: &str, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table
            .filter(nanoid_field.eq(nano))
            .first(conn)
            .await?)
    }

    pub async fn invitees(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentInvitation::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn players(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn number_of_players(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn number_of_games(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(games::table
            .filter(games::tournament_id.eq(self.id))
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn number_of_finished_games(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(games::table
            .filter(
                games::tournament_id
                    .eq(self.id)
                    .and(games::finished.eq(true)),
            )
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn organizers(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentOrganizer::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn games(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(tournament_id_column.eq(Some(self.id)))
            .get_results(conn)
            .await?)
    }

    pub async fn get_standings(&self, conn: &mut DbConn<'_>) -> Result<Standings, DbError> {
        let players = self.players(conn).await?;
        let games = self.games(conn).await?;
        let mut standings = Standings::new();

        // Add tiebreakers from tournament configuration
        for tiebreaker in self.tiebreaker.iter().flatten() {
            standings.add_tiebreaker(Tiebreaker::from_str(tiebreaker).map_err(|e| {
                DbError::InvalidInput {
                    info: String::from("Invalid tiebreaker"),
                    error: e.to_string(),
                }
            })?);
        }

        // Add all games to standings
        for game in games.iter() {
            standings.add_result(
                game.white_id,
                game.black_id,
                game.white_rating.unwrap_or(0.0),
                game.black_rating.unwrap_or(0.0),
                TournamentGameResult::from_str(&game.tournament_game_result).map_err(|e| {
                    DbError::InvalidInput {
                        info: String::from("Invalid game result"),
                        error: e.to_string(),
                    }
                })?,
            );
        }

        // Handle byes (players who didn't play in any games)
        let players_with_games: HashSet<Uuid> = games
            .iter()
            .flat_map(|g| [g.white_id, g.black_id])
            .collect();

        for player in &players {
            if !players_with_games.contains(&player.id) {
                println!("Adding bye for player {}", player.username);
                // Add a bye as an automatic win
                standings.add_result(
                    player.id,
                    player.id, // Self-play indicates a bye
                    0.0,       // Rating doesn't matter for byes
                    0.0,
                    TournamentGameResult::Bye,
                );
            }
        }

        // Calculate all tiebreakers
        standings.enforce_tiebreakers();

        Ok(standings)
    }

    pub async fn start_by_organizer(
        &self,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        self.ensure_user_is_organizer(organizer, conn).await?;
        self.start(conn).await
    }

    pub async fn start(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        println!("Starting tournament {} ({})", self.name, self.id);
        self.ensure_not_started()?;
        if !self.has_enough_players(conn).await? {
            println!("Not enough players to start tournament");
            return Err(DbError::NotEnoughPlayers);
        }
        println!("Tournament has enough players, proceeding with start");

        let ends = if let Some(days) = self.round_duration {
            let days = TimeDelta::days(days as i64);
            Some(Utc::now() + days)
        } else {
            None
        };
        println!("Tournament end time: {:?}", ends);

        // Make sure all the conditions have been met
        // and then call different starts for different tournament types
        let mut deleted_invitees = Vec::new();
        println!("Starting tournament in {} mode", self.mode);
        let games = match self.mode.to_uppercase().as_str() {
            "SWISS" => self.swiss_start(conn).await?,
            "RR" => self.round_robin_start(conn).await?,
            _ => {
                println!("Invalid tournament mode: {}", self.mode);
                return Err(DbError::InvalidInput {
                    info: format!("Unsupported tournament mode: {}", self.mode),
                    error: String::from("Tournament mode must be either SWISS or RR"),
                });
            }
        };
        println!("Created {} games for tournament", games.len());

        let tournament: Tournament = diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Utc::now()),
                ends_at.eq(ends),
                current_round.eq(1),
            ))
            .get_result(conn)
            .await?;
        println!("Tournament status updated to InProgress");

        let invitations: Vec<TournamentInvitation> = TournamentInvitation::belonging_to(self)
            .get_results(conn)
            .await?;
        println!(
            "Found {} pending invitations to clean up",
            invitations.len()
        );
        for invitation in invitations.iter() {
            deleted_invitees.push(invitation.invitee_id);
            invitation.delete(conn).await?;
        }
        println!("Cleaned up all pending invitations");

        Ok((tournament, games, deleted_invitees))
    }

    /// Finds a player to receive a bye based on initial seeding and previous byes.
    /// Returns None if no eligible player is found.
    /// The function prioritizes players who:
    /// 1. Haven't received a bye yet
    /// 2. Are ranked lower in the initial seeding
    pub fn find_bye_player(&self) -> Option<Uuid> {
        // Get the set of players who have already received byes
        let players_with_byes: HashSet<_> = self.bye.iter().flatten().collect();

        // Start from the end of initial_seeding (lowest ranked players)
        // and find the first player who hasn't had a bye yet
        let eligible_players: Vec<_> = self
            .initial_seeding
            .iter()
            .rev() // Reverse to start from lowest ranked
            .flatten() // Remove None values
            .filter(|player_id| !players_with_byes.contains(player_id))
            .collect();

        if eligible_players.is_empty() {
            // If all players have had byes, just take the lowest ranked player
            self.initial_seeding.last().and_then(|x| *x)
        } else {
            // If there are multiple eligible players with the same lowest rank,
            // randomly select one of them
            Some(*eligible_players[random::<usize>() % eligible_players.len()])
        }
    }

    pub async fn swiss_start(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        println!("Starting Swiss tournament initialization");
        let mut games = Vec::new();
        let players = self.players(conn).await?;
        println!("Found {} players for Swiss tournament", players.len());

        // Determine game speed based on tournament time mode
        let game_speed = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => GameSpeed::Blitz,
            TimeMode::Untimed => {
                println!("Cannot start untimed tournament");
                return Err(DbError::InvalidInput {
                    info: String::from("Cannot start untimed tournament"),
                    error: String::from("Tournament must have a time mode"),
                });
            }
        };
        println!("Using game speed: {:?}", game_speed);

        // Sort players by rating for initial seeding
        let mut players_with_ratings: Vec<(User, f64)> = Vec::new();
        for player in players {
            let rating = Rating::for_uuid(&player.id, &game_speed, conn).await?;
            players_with_ratings.push((player, rating.rating));
        }
        players_with_ratings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Store initial seeding
        let initial_seeding: Vec<Option<Uuid>> = players_with_ratings
            .iter()
            .map(|(player, _)| Some(player.id))
            .collect();
        println!("Initial seeding: {:?}", initial_seeding);
        diesel::update(self)
            .set((
                tournaments::initial_seeding.eq(initial_seeding),
                tournament::current_round.eq(self.current_round + 1),
            ))
            .execute(conn)
            .await?;

        // Handle odd number of players first
        let mut players_to_pair = players_with_ratings.clone();
        if players_to_pair.len() % 2 != 0 {
            if let Some(bye_player_id) = self.find_bye_player() {
                let bye_player_idx = players_to_pair
                    .iter()
                    .position(|(p, _)| p.id == bye_player_id)
                    .unwrap();
                let bye_player = players_to_pair.remove(bye_player_idx);
                println!(
                    "Odd number of players, giving bye to {} (rating: {})",
                    bye_player.0.username, bye_player.1
                );

                diesel::update(tournaments::table.find(self.id))
                    .set(bye.eq(vec![Some(bye_player.0.id)]))
                    .execute(conn)
                    .await?;

                // Create a bye game
                let new_game = NewGame::new_from_tournament(bye_player.0.id, bye_player.0.id, self);
                let game = Game::create(new_game, conn).await?;
                games.push(game);
            }
        } else {
            diesel::update(self)
                .set(bye.eq(vec![None::<Uuid>]))
                .execute(conn)
                .await?;
        }

        // Apply pairing rules based on seeding mode
        let seeding_mode = SeedingMode::from_str(&self.seeding_mode)?;
		let remaining_players = players_to_pair.len();
		let quarter_size = remaining_players / 4; // Note that remaining_players is an integer. So / here is integer division!

		let pairing_ranges = match seeding_mode{
			SeedingMode::Accelerated => vec![
				(0, quarter_size * 2),                  // Q1 vs Q2 Using quarter_size guaranteees even players
				(quarter_size * 2, remaining_players),  // Q3 vs Q4 (guaranteed even due to fixed bye)
				],
			SeedingMode::Standard => vec![(0,remaining_players)],
			};
		
		for &(start, end) in &pairing_ranges {
			let mid = start + (end - start) / 2; 

			for i in start..mid {
				let white = &players_to_pair[i].0;      // Top half
				let black = &players_to_pair[mid + (i - start)].0; // Bottom half

				println!(
					"  Pairing {} (White, rating: {}) vs {} (Black, rating: {})",
					white.username, players_to_pair[i].1,
					black.username, players_to_pair[mid + (i - start)].1
				);

				let new_game = NewGame::new_from_tournament(white.id, black.id, self);
				let game = Game::create(new_game, conn).await?;
				games.push(game);
			}
		}

        println!(
            "Swiss tournament initialization complete with {} games",
            games.len()
        );
        Ok(games)
    }

    pub async fn round_robin_start(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        let mut games = Vec::new();
        let players = self.players(conn).await?;
        let combinations: Vec<Vec<User>> = players.into_iter().combinations(2).collect();
        for combination in combinations {
            let white = combination[0].id;
            let black = combination[1].id;
            let new_game = NewGame::new_from_tournament(white, black, self);
            let game = Game::create(new_game, conn).await?;
            games.push(game);
            let new_game = NewGame::new_from_tournament(black, white, self);
            let game = Game::create(new_game, conn).await?;
            games.push(game);
        }
        Ok(games)
    }

    pub async fn find(id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(tournaments::table.find(id).first(conn).await?)
    }

    pub async fn find_by_tournament_id(
        tournament_id: &TournamentId,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        let TournamentId(id) = tournament_id;
        Ok(tournaments::table
            .filter(nanoid_field.eq(id))
            .first(conn)
            .await?)
    }

    pub async fn unstarted(conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        let potential_tournaments: Vec<Tournament> = tournaments::table
            .filter(status_column.eq(TournamentStatus::NotStarted.to_string()))
            .filter(starts_at.le(Utc::now()))
            .get_results(conn)
            .await?;
        let mut tournaments = Vec::new();
        for tournament in potential_tournaments {
            if tournament.has_enough_players(conn).await? {
                tournaments.push(tournament);
            }
        }
        Ok(tournaments)
    }

    pub async fn automatic_start(
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Tournament, Vec<Game>, Vec<Uuid>)>, DbError> {
        let mut started_tournaments = Vec::new();
        for tournament in Tournament::unstarted(conn).await? {
            started_tournaments.push(tournament.start(conn).await?);
        }
        Ok(started_tournaments)
    }

    pub async fn get_all(
        sort_order: TournamentSortOrder,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        let query = tournaments::table.into_boxed();
        let sorted_query = match sort_order {
            TournamentSortOrder::CreatedAtDesc => query.order(tournaments::created_at.desc()),
            TournamentSortOrder::CreatedAtAsc => query.order(tournaments::created_at.asc()),
            TournamentSortOrder::StartedAtDesc => query.order(tournaments::started_at.desc()),
            TournamentSortOrder::StartedAtAsc => query.order(tournaments::started_at.asc()),
        };
        Ok(sorted_query.get_results(conn).await?)
    }

	//Return a hashmap of player_id to score
	pub async fn get_player_info(
			&self,
	        conn: &mut DbConn<'_>,
		) -> Result<HashMap<Uuid, (f64, HashSet<Uuid>)>, DbError> { 
		let mut player_info = HashMap::new();
		
        let players = self.players(conn).await?;
        let existing_games = self.games(conn).await?;
        println!(
            "Found {} players and {} existing games",
            players.len(),
            existing_games.len()
        );
		
        println!("\nCalculating current scores and previous opponents:");

        for player in &players {
            let mut score = 0.0;
            let mut opponents = HashSet::new();

            for game in &existing_games {
                if game.white_id == player.id {
                    opponents.insert(game.black_id);
                    match TournamentGameResult::from_str(&game.tournament_game_result) {
                        Ok(TournamentGameResult::Winner(Color::White)) => score += 1.0,
                        Ok(TournamentGameResult::Draw) => score += 0.5,
                        _ => {}
                        Err(_) => {
                            return Err(DbError::InvalidInput {
                                info: format!(
                                    "Invalid game result: {}",
                                    game.tournament_game_result
                                ),
                                error: String::from("Failed to parse tournament game result"),
                            })
                        }
                    }
                } else if game.black_id == player.id {
                    opponents.insert(game.white_id);
                    match TournamentGameResult::from_str(&game.tournament_game_result) {
                        Ok(TournamentGameResult::Winner(Color::Black)) => score += 1.0,
                        Ok(TournamentGameResult::Draw) => score += 0.5,
                        _ => {}
                        Err(_) => {
                            return Err(DbError::InvalidInput {
                                info: format!(
                                    "Invalid game result: {}",
                                    game.tournament_game_result
                                ),
                                error: String::from("Failed to parse tournament game result"),
                            })
                        }
                    }
                }
            }

            println!(
                "  Player {} has {:.1} points and {} previous opponents",
                players.iter().find(|p| p.id == player.id).unwrap().username,
                score,
                opponents.len()
            );
            player_info.insert(player.id, (score, opponents));
        }
		return Ok(player_info)
	}

    pub async fn create_next_round(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Self, Vec<Game>), DbError> {
        if self.mode.to_uppercase() != "SWISS" {
            return Err(DbError::InvalidInput {
                info: String::from("Not a Swiss tournament"),
                error: String::from("Can only create next round for Swiss tournaments"),
            });
        }

        println!(
            "\nStarting next round creation for tournament {} ({})",
            self.name, self.id
        );
        let mut games = Vec::<Game>::new();
        let players = self.players(conn).await?;

        // Create a map of player scores and opponents
        let mut player_info: HashMap<Uuid, (f64, HashSet<Uuid>)> = self.get_player_info(conn).await.unwrap();

        // Sort players by score
        let mut players_to_pair: Vec<(User, f64)> = players
            .iter()
            .map(|p| (p.clone(), player_info.get(&p.id).unwrap().0))
            .collect();
        players_to_pair.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        println!("\nPlayers sorted by score:");
        for (player, score) in &players_to_pair {
            println!("  {}: {:.1} points", player.username, score);
        }

        // Handle odd number of players
        if players_to_pair.len() % 2 != 0 {
            println!(
                "\nOdd number of players ({}), looking for bye candidate",
                players_to_pair.len()
            );

            // Find bye player using the new function
            if let Some(bye_player_id) = self.find_bye_player() {
                let bye_player_idx = players_to_pair
                    .iter()
                    .position(|(p, _)| p.id == bye_player_id)
                    .unwrap();
                let bye_player = players_to_pair.remove(bye_player_idx);
                println!(
                    "  Giving bye to {} (score: {:.1})",
                    bye_player.0.username, bye_player.1
                );

                // Update tournament to track the new bye
                let mut new_byes = self.bye.clone();
                new_byes.push(Some(bye_player.0.id));
                diesel::update(tournaments::table.find(self.id))
                    .set(bye.eq(new_byes))
                    .execute(conn)
                    .await?;
                println!(
                    "  Updated tournament to track bye for player {} in round {}",
                    bye_player.0.username,
                    self.current_round + 1
                );
            }
        } else {
            println!("Adding another None to tournament.bye");
            let mut new_byes = self.bye.clone();
            new_byes.push(None);
            diesel::update(tournaments::table.find(self.id))
                .set(bye.eq(new_byes))
                .execute(conn)
                .await?;
        }

        // Create pairings for remaining players
        println!("\nCreating pairings for remaining players:");
        while !players_to_pair.is_empty() {
            let mut paired = false;
            let current_player = &players_to_pair[0];
            let current_opponents = &player_info.get(&current_player.0.id).unwrap().1;

            println!(
                "  Looking for opponent for {} ({:.1} points)",
                current_player.0.username, current_player.1
            );

            // Try to find an opponent
            for i in 1..players_to_pair.len() {
                let potential_opponent = &players_to_pair[i];
                if !current_opponents.contains(&potential_opponent.0.id) {
                    println!(
                        "    Found valid opponent: {} ({:.1} points)",
                        potential_opponent.0.username, potential_opponent.1
                    );
                    // Create the game
                    let white = players_to_pair.remove(0).0;
                    let black = players_to_pair.remove(i - 1).0;
                    println!(
                        "    Creating game: {} (White) vs {} (Black)",
                        white.username, black.username
                    );
                    let new_game = NewGame::new_from_tournament(white.id, black.id, self);
                    let game = Game::create(new_game, conn).await?;
                    games.push(game);
                    paired = true;
                    break;
                } else {
                    println!(
                        "    {} already played against {}, skipping",
                        current_player.0.username, potential_opponent.0.username
                    );
                }
            }

            if !paired {
                // If no opponent found, pair with the next available player
                let white = players_to_pair.remove(0).0;
                let black = players_to_pair.remove(0).0;
                println!(
                    "    No valid opponents found, forced pairing: {} vs {}",
                    white.username, black.username
                );
                let new_game = NewGame::new_from_tournament(white.id, black.id, self);
                let game = Game::create(new_game, conn).await?;
                games.push(game);
            }
        }

        // Increment the current round
        let tournament = diesel::update(self)
            .set(current_round.eq(self.current_round + 1))
            .get_result::<Tournament>(conn)
            .await?;

        println!(
            "\nRound {} creation complete - created {} games",
            tournament.current_round,
            games.len()
        );
        Ok((tournament, games))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rating::NewRating;
    use crate::models::user::NewUser;
    use crate::schema::ratings;
    use crate::schema::users;
    use crate::{config::DbConfig, get_conn, get_pool};
    use diesel::Connection;
    use diesel_async::AsyncConnection;
    use diesel_async::AsyncPgConnection;
    use std::pin::Pin;

    type BoxedDbConn = Pin<Box<DbConn<'static>>>;

    async fn setup_test_db() -> Result<BoxedDbConn, DbError> {
        let config = DbConfig::from_test_env().map_err(|e| DbError::InvalidInput {
            info: String::from("Failed to get test database config"),
            error: e.to_string(),
        })?;

        let pool = get_pool(&config.database_url)
            .await
            .map_err(|e| DbError::InvalidInput {
                info: String::from("Failed to get database pool"),
                error: e.to_string(),
            })?;

        // Leak the pool to extend its lifetime to 'static
        let pool = Box::leak(Box::new(pool));

        let mut conn = get_conn(pool).await.map_err(|e| DbError::InvalidInput {
            info: String::from("Failed to get database connection"),
            error: e.to_string(),
        })?;
        conn.begin_test_transaction().await?;

        // Convert the connection into a pinned box
        Ok(Box::pin(conn))
    }

    async fn create_test_tournament(
        conn: &mut BoxedDbConn,
        num_players: i32,
        seeding_mode: SeedingMode,
    ) -> Result<(Tournament, Vec<User>), DbError> {
        // Create tournament details
        let tournament_details = TournamentDetails {
            name: format!("Test {} Swiss Tournament", seeding_mode.to_string()),
            description: "A test tournament".to_string(),
            scoring: ScoringMode::Game,
            tiebreakers: vec![
                Some(Tiebreaker::Buchholz),
                Some(Tiebreaker::BuchholzCut1),
                Some(Tiebreaker::WinsAsBlack),
                Some(Tiebreaker::DirectEncounter),
            ],
            seats: num_players,
            min_seats: num_players - 2,
            rounds: 5,
            invite_only: false,
            mode: "SWISS".to_string(),
            time_mode: TimeMode::Correspondence,
            time_base: Some(1),
            time_increment: None,
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual,
            starts_at: None,
            round_duration: None,
            series: None,
            invitees: vec![],
            seeding_mode: Some(seeding_mode),
        };

        // Create organizer and tournament
        let new_tournament = NewTournament::new(tournament_details)?;
        let new_user = NewUser {
            username: String::from("TournamentOrganizer"),
            password: "test_hash".to_string(),
            email: String::from("org@test.com"),
            normalized_username: String::from("tournamentorganizer"),
            patreon: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let organizer = diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(conn)
            .await?;
        let tournament = Tournament::create(organizer.id, &new_tournament, conn).await?;

        // Create players with different ratings
        let mut players = Vec::new();
        for i in 0..num_players {
            let new_user = NewUser {
                username: format!("player{}", i + 1),
                password: "test_hash".to_string(),
                email: format!("player{}@test.com", i + 1),
                normalized_username: format!("player{}", i + 1).to_lowercase(),
                patreon: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let user = diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<User>(conn)
                .await?;

            // Create rating (ranging from 1200 to 2800)
            let rating_value = 1200.0 + (i as f64 * 100.0);
            let new_rating = NewRating {
                user_uid: user.id,
                played: 0,
                won: 0,
                lost: 0,
                draw: 0,
                rating: rating_value,
                deviation: 350.0,
                volatility: 0.06,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                speed: GameSpeed::Correspondence.to_string(),
            };
            diesel::insert_into(ratings::table)
                .values(&new_rating)
                .execute(conn)
                .await?;

            tournament.join(&user.id, conn).await?;
            players.push(user);
        }

        Ok((tournament, players))
    }

    #[tokio::test]
    async fn test_swiss_accelerated_odd_players() -> Result<(), DbError> {
        println!(
            "Starting Swiss tournament test with accelerated seeding and odd number of players..."
        );
        let mut conn = setup_test_db().await?;

        // Create tournament with 15 players and accelerated seeding
        let (tournament, players) =
            create_test_tournament(&mut conn, 15, SeedingMode::Accelerated).await?;
        println!(
            "Created tournament with {} players and accelerated seeding",
            players.len()
        );

        // Start tournament and verify initial state
        let (tournament, initial_games, _) = tournament.start(&mut conn).await?;
        println!(
            "Tournament started with {} initial games",
            initial_games.len()
        );
        assert_eq!(
            initial_games.len(),
            7,
            "Expected 7 games + 1 bye in first round"
        );

        // Verify the bye was given to the highest rated player (player1)
        let bye_game = initial_games.iter().find(|g| g.white_id == g.black_id);
        assert!(bye_game.is_some(), "No bye game found");
        let bye_player = players
            .iter()
            .find(|p| p.id == bye_game.unwrap().white_id)
            .unwrap();
        assert_eq!(
            bye_player.username, "player1",
            "Bye should be given to highest rated player"
        );

        // Verify correct accelerated pairings
        // In order of rating (high to low): player7 vs player4, player6 vs player3, player5 vs player2
        let mut found_pairings = vec![];
        for game in initial_games.iter().filter(|g| g.white_id != g.black_id) {
            let white = players.iter().find(|p| p.id == game.white_id).unwrap();
            let black = players.iter().find(|p| p.id == game.black_id).unwrap();
            found_pairings.push((white.username.clone(), black.username.clone()));
            println!("Found pairing: {} vs {}", white.username, black.username);
        }

        let expected_pairings = vec![
            ("player7".to_string(), "player4".to_string()),
            ("player6".to_string(), "player3".to_string()),
            ("player5".to_string(), "player2".to_string()),
            ("player11".to_string(), "player8".to_string()),
            ("player10".to_string(), "player13".to_string()),
            ("player9".to_string(), "player12".to_string()),
            ("player15".to_string(), "player14".to_string()),
        ];

        assert_eq!(
            found_pairings.len(),
            expected_pairings.len(),
            "Wrong number of pairings"
        );
        for expected in expected_pairings {
            assert!(
                found_pairings.contains(&expected),
                "Missing expected pairing: {} vs {}",
                expected.0,
                expected.1
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_swiss_standard_even_players() -> Result<(), DbError> {
        println!(
            "Starting Swiss tournament test with standard seeding and even number of players..."
        );
        let mut conn = setup_test_db().await?;

        // Create tournament with 16 players and standard seeding
        let (mut tournament, players) =
            create_test_tournament(&mut conn, 16, SeedingMode::Standard).await?;
        println!(
            "Created tournament with {} players and standard seeding",
            players.len()
        );

        let mut games = Vec::new();

        // Simulate 5 rounds
        for round in 1..=5 {
            println!("\n=== Round {} ===", round);
            (tournament, games) = if round == 1 {
                // Start tournament and verify initial state
                let (tournament, initial_games, _) = tournament.start(&mut conn).await?;
                println!(
                    "Tournament started with {} initial games",
                    initial_games.len()
                );
                assert_eq!(initial_games.len(), 8, "Expected 8 games in first round");
                assert_eq!(
                    tournament.current_round, 1,
                    "Current_round is 1 in first round"
                );
                (tournament, initial_games)
            } else {
                tournament.create_next_round(&mut conn).await?
            };

            assert_eq!(tournament.bye, vec![None; round as usize]);
            assert_eq!(
                tournament.current_round, round,
                "Current round and round don't match"
            );
            // In an even tournament no one gets a bye

            assert_eq!(games.len(), 8, "Expected 8 games in each round");

            // Simulate game results
            for game in &games {
                let white_rating =
                    Rating::for_uuid(&game.white_id, &GameSpeed::Correspondence, &mut conn)
                        .await?
                        .rating;
                let black_rating =
                    Rating::for_uuid(&game.black_id, &GameSpeed::Correspondence, &mut conn)
                        .await?
                        .rating;

                // Mix of results to make it interesting
                let result = match (round as i32 + game.id.as_u128() as i32) % 3 {
                    0 => TournamentGameResult::Winner(Color::White),
                    1 => TournamentGameResult::Winner(Color::Black),
                    _ => TournamentGameResult::Draw,
                };

                diesel::update(games::table.find(game.id))
                    .set((
                        games::tournament_game_result.eq(result.to_string()),
                        games::finished.eq(true),
                    ))
                    .execute(&mut conn)
                    .await?;
            }

            // Verify no byes in any round
            let round_games = tournament.games(&mut conn).await?;
            assert!(
                round_games.iter().all(|g| g.white_id != g.black_id),
                "Found unexpected bye in round {}",
                round
            );
        }

        // Verify final standings have all players
        let final_standings = tournament.get_standings(&mut conn).await?;
        assert_eq!(
            final_standings.players_standings.len(),
            16,
            "Expected all 16 players in final standings"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_swiss_tournament_simulation() -> Result<(), DbError> {
        println!("Starting Swiss tournament simulation test...");
        let mut conn = setup_test_db().await?;

        // Create a Swiss tournament
        let tournament_details = TournamentDetails {
            name: "Test Swiss Tournament".to_string(),
            description: "A test tournament".to_string(),
            scoring: ScoringMode::Game,
            tiebreakers: vec![
                Some(Tiebreaker::Buchholz),
                Some(Tiebreaker::BuchholzCut1),
                Some(Tiebreaker::WinsAsBlack),
                Some(Tiebreaker::DirectEncounter),
            ],
            seats: 16,
            min_seats: 8,
            rounds: 5,
            invite_only: false,
            mode: "SWISS".to_string(),
            time_mode: TimeMode::Correspondence,
            time_base: Some(1),
            time_increment: None,
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual,
            starts_at: None,
            round_duration: None,
            series: None,
            invitees: vec![],
            seeding_mode: Some(SeedingMode::Standard),
        };

        let new_tournament = NewTournament::new(tournament_details)?;
        println!("New tournament");
        let new_user = NewUser {
            username: String::from("TournamentOrganizer"),
            password: "test_hash".to_string(),
            email: String::from("org@test.com"),
            normalized_username: String::from("tournamentorganizer"),
            patreon: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let organizer = diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(&mut conn)
            .await?;
        let tournament = Tournament::create(organizer.id, &new_tournament, &mut conn).await?;
        println!("Tournament created with ID: {}", tournament.id);

        // Create 15 players with different ratings
        let mut players = Vec::new();
        for i in 0..15 {
            let new_user = NewUser {
                username: format!("player{}", i + 1),
                password: "test_hash".to_string(),
                email: format!("player{}@test.com", i + 1),
                normalized_username: format!("player{}", i + 1).to_lowercase(),
                patreon: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let user = diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<User>(&mut conn)
                .await?;
            println!("Created player {} with ID: {}", user.username, user.id);

            // Create a rating for the player (ranging from 1200 to 2800)
            let rating_value = 1200.0 + (i as f64 * 100.0);
            let new_rating = NewRating {
                user_uid: user.id,
                played: 0,
                won: 0,
                lost: 0,
                draw: 0,
                rating: rating_value,
                deviation: 350.0,
                volatility: 0.06,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                speed: GameSpeed::Correspondence.to_string(),
            };
            diesel::insert_into(ratings::table)
                .values(&new_rating)
                .execute(&mut conn)
                .await?;
            println!("Created rating for {}: {}", user.username, rating_value);

            // Add player to tournament
            tournament.join(&user.id, &mut conn).await?;
            players.push(user);
        }
        println!("All players created and joined tournament");

        // Start the tournament
        let (tournament, initial_games, _) = tournament.start(&mut conn).await?;
        println!(
            "Tournament started with {} initial games",
            initial_games.len()
        );
        debug_assert_eq!(
            initial_games.len(),
            7,
            "Expected 8 initial games (7 games + 1 bye)"
        );

        // Simulate 5 rounds
        for round in 1..=5 {
            println!("\n=== Simulating round {} ===", round);

            // Get games for this round
            let (tournament, games) = if round == 1 {
                (tournament.clone(), initial_games.clone())
            } else {
                tournament.create_next_round(&mut conn).await?
            };
            println!("Round {} has {} games", round, games.len());

            // Simulate results for each game
            for game in &games {
                let white = User::find_by_uuid(&game.white_id, &mut conn).await?;
                let black = User::find_by_uuid(&game.black_id, &mut conn).await?;
                println!(
                    "Processing game {}: {} vs {}",
                    game.id, white.username, black.username
                );

                let result = if game.white_id == game.black_id {
                    println!("  This is a bye for player {}", game.white_id);
                    TournamentGameResult::Bye
                } else {
                    // Randomly determine winner (biased towards higher rated player)
                    let white_rating =
                        Rating::for_uuid(&game.white_id, &GameSpeed::Correspondence, &mut conn)
                            .await?
                            .rating;
                    let black_rating =
                        Rating::for_uuid(&game.black_id, &GameSpeed::Correspondence, &mut conn)
                            .await?
                            .rating;

                    println!(
                        "  Ratings - White: {}, Black: {}",
                        white_rating, black_rating
                    );

                    let random = random::<f64>();
                    let white_win_prob = 0.5 + (white_rating - black_rating) / 2000.0;
                    println!("  Win probability for White: {:.2}", white_win_prob);

                    let result = if random < white_win_prob {
                        TournamentGameResult::Winner(Color::White)
                    } else if random < white_win_prob + 0.1 {
                        TournamentGameResult::Draw
                    } else {
                        TournamentGameResult::Winner(Color::Black)
                    };
                    println!("  Result: {:?}", result);
                    result
                };

                // Update game result
                diesel::update(games::table.find(game.id))
                    .set((
                        games::tournament_game_result.eq(result.to_string()),
                        games::finished.eq(true),
                    ))
                    .execute(&mut conn)
                    .await?;
                println!("  Game result updated in database");
            }

            // Print standings after each round
            let standings = tournament.get_standings(&mut conn).await?;
            println!("\nStandings after round {}:", round);

            for (i, standing) in standings.players_standings.iter().enumerate() {
                for player_id in standing {
                    let player = players.iter().find(|p| p.id == *player_id).unwrap();
                    let score = standings
                        .players_scores
                        .get(player_id)
                        .unwrap()
                        .get(&Tiebreaker::RawPoints)
                        .unwrap();
                    println!("{}. {}: {:.1} points", i + 1, player.username, score);
                }
            }

            // Verify round state
            let games_count = tournament.number_of_games(&mut conn).await?;
            let finished_games_count = tournament.number_of_finished_games(&mut conn).await?;
            println!("\nRound {} verification:", round);
            println!("  Total games: {}", games_count);
            println!("  Finished games: {}", finished_games_count);
            debug_assert_eq!(
                games_count, finished_games_count,
                "Not all games in round {} are finished",
                round
            );
        }

        // Verify final standings
        let final_standings = tournament.get_standings(&mut conn).await?;
        println!("\n=== Final Standings ===");
        println!(
            "Number of standings groups: {}",
            final_standings.players_standings.len()
        );
        debug_assert_eq!(
            final_standings.players_standings.len(),
            15,
            "Expected 15 players in final standings"
        );

        // Verify all games are finished
        let total_games = tournament.number_of_games(&mut conn).await?;
        let finished_games = tournament.number_of_finished_games(&mut conn).await?;
        println!("\nFinal verification:");
        println!("  Total games: {}", total_games);
        println!("  Finished games: {}", finished_games);
        debug_assert_eq!(
            total_games, finished_games,
            "Not all games are finished at the end of the tournament"
        );

        // Verify no player has more than one bye
        let mut bye_counts: HashMap<Uuid, i32> = HashMap::new();
        for game in tournament.games(&mut conn).await? {
            if game.white_id == game.black_id {
                *bye_counts.entry(game.white_id).or_insert(0) += 1;
            }
        }
        println!("\nBye counts:");
        for (player_id, count) in &bye_counts {
            let player = players.iter().find(|p| p.id == *player_id).unwrap();
            println!("  {}: {} byes", player.username, count);
            debug_assert!(
                *count <= 1,
                "Player {} received more than one bye",
                player.username
            );
        }

        println!("\nTest completed successfully!");
        Ok(())
    }
}
