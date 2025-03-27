use super::{Game, NewGame, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
        rating::Rating,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        tournaments::{
            self, ends_at, nanoid as nanoid_field, series as series_column, started_at, starts_at,
            status as status_column, updated_at, current_round,
        },
        tournaments_organizers, users,
    },
    DbConn,
};
use chrono::{prelude::*, TimeDelta};
use diesel::prelude::*;
use diesel_async::{RunQueryDsl, AsyncConnection};
use itertools::Itertools;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{
    Certainty, GameSpeed, SeedingMode, TimeMode, TournamentDetails, TournamentId, TournamentSortOrder, TournamentStatus,
    ScoringMode, StartMode, Tiebreaker,
};
use uuid::Uuid;
use chrono::{Duration, Utc};
use std::fmt::Write;

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
    pub games_per_round: i32,
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
            games_per_round: details.games_per_round,
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
    pub games_per_round: i32,
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
        self.ensure_not_started()?;
        if !self.has_enough_players(conn).await? {
            return Err(DbError::NotEnoughPlayers);
        }
        let ends = if let Some(days) = self.round_duration {
            let days = TimeDelta::days(days as i64);
            Some(Utc::now() + days)
        } else {
            None
        };
        // Make sure all the conditions have been met
        // and then call different starts for different tournament types
        let mut deleted_invitees = Vec::new();
        let games = self.round_robin_start(conn).await?;
        let tournament: Tournament = diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Utc::now()),
                ends_at.eq(ends),
            ))
            .get_result(conn)
            .await?;
        let invitations: Vec<TournamentInvitation> = TournamentInvitation::belonging_to(self)
            .get_results(conn)
            .await?;
        for invitation in invitations.iter() {
            deleted_invitees.push(invitation.invitee_id);
            invitation.delete(conn).await?;
        }
        Ok((tournament, games, deleted_invitees))
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

    pub async fn generate_initial_seeding(&self, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        let players = self.players(conn).await?;
        println!("Setting up initial seeding for Swiss tournament with {} players", players.len());

        // Create a mapping of GameSpeed from tournament time settings
        let game_speed = GameSpeed::from_base_increment(self.time_base, self.time_increment);
        println!("Tournament game speed: {}", game_speed);

        // Get player ratings and information
        let mut player_ratings = Vec::new();
        for player in &players {
            let rating = Rating::for_uuid(&player.id, &game_speed, conn).await?;
            
            // Determine certainty level from deviation
            let certainty = Certainty::from_deviation(rating.deviation);
            
            println!("Player {}: rating={}, deviation={}, certainty={:?}", 
                     player.username, rating.rating, rating.deviation, &certainty);
                     
            player_ratings.push((player.id, rating.rating as u64, certainty));
        }

        // Sort players: first by certainty level, then by rating
        player_ratings.sort_by(|a, b| {
            // First sort by certainty (using its Ord implementation)
            match a.2.cmp(&b.2) {
                std::cmp::Ordering::Equal => {
                    // If certainty is equal, sort by rating (higher first)
                    b.1.cmp(&a.1) 
                },
                other_ordering => other_ordering
            }
        });

        // Create the initial seeding array
        let initial_seeding: Vec<Option<Uuid>> = player_ratings.iter()
            .map(|(uuid, _, _)| Some(*uuid))
            .collect();

        // Update tournament with initial seeding and return the updated instance
        let updated = diesel::update(self)
            .set(tournaments::initial_seeding.eq(&initial_seeding))
            .get_result(conn)
            .await?;

        println!("Initial seeding set for Swiss tournament");
        Ok(updated)
    }
    
    pub async fn swiss_start(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        // 1. Generate initial seeding
        let tournament = self.generate_initial_seeding(conn).await?;
        println!("Initial seeding generated successfully");
    
        // 2. Generate the TRFx file and write to disk
        let trfx_file_path = tournament.save_trfx(conn).await?;
        println!("TRFx file saved to: {}", trfx_file_path);
        
        // 3. Generate pairings using external program
        let pairings_file_path = tournament.generate_pairings(&trfx_file_path)?;
        println!("Pairings generated and saved to: {}", pairings_file_path);
        
        // 4. Read the pairings and create games
        let games = tournament.create_games_from_pairing_file(&pairings_file_path, conn).await?;
        println!("Created {} games from pairings", games.len());
        
        // 5. Update tournament to indicate first round is created
        diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Utc::now()),
                current_round.eq(1), // Set to round 1
            ))
            .execute(conn)
            .await?;
        
        Ok(games)
    }

    /// Generate pairings using the external pairing program
    pub fn generate_pairings(&self, trfx_file_path: &str) -> Result<String, DbError> {
        // Get the pairer executable path based on OS
        let pairer_path = if cfg!(target_os = "macos") {
            "/Users/leex/src/hive/tools/macos/pairer"
        } else {
            "/Users/leex/src/hive/tools/linux/pairer"
        };

        // Generate output file path
        let output_file_path = trfx_file_path.replace(".trfx", "_pairing");
        println!("Pairing output will be saved to: {}", output_file_path);

        // Execute pairer
        println!("Executing pairer: {} --dutch {} -p {}", pairer_path, trfx_file_path, output_file_path);
        let output = std::process::Command::new(pairer_path)
            .arg("--dutch")
            .arg(trfx_file_path)
            .arg("-p")
            .arg(&output_file_path)
            .output()
            .map_err(|e| {
                DbError::InvalidInput {
                    info: "Failed to execute pairer".to_string(),
                    error: e.to_string(),
                }
            })?;

        // Print the error output if any
        if !output.stderr.is_empty() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            println!("Pairer error output: {}", error_msg);
        }

        if !output.status.success() {
            return Err(DbError::InvalidInput {
                info: "Pairing program failed".to_string(),
                error: format!("Exit code: {:?}\nError: {}", 
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)),
            });
        }

        Ok(output_file_path)
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

    pub async fn generate_trfx(&self, conn: &mut DbConn<'_>) -> Result<String, DbError> {
        println!("Starting TRFx generation for tournament: {}", self.name);
        println!("Tournament mode: {}", self.seeding_mode);
        
        let players = self.players(conn).await?;
        println!("Found {} players", players.len());
        
        let games = self.games(conn).await?;
        println!("Found {} games", games.len());
        
        // Build header section
        let mut trfx = String::new();
        writeln!(trfx, "012 {}", self.name)?;
        writeln!(trfx, "022 Hivegame.com")?;
        writeln!(trfx, "032 Hiveystan")?;
        writeln!(trfx, "042 {}", Utc::now().format("%Y-%m-%d"))?;
        writeln!(trfx, "052 {}", Utc::now().format("%Y-%m-%d"))?;
        writeln!(trfx, "062 {}", players.len())?;
        writeln!(trfx, "072 0")?;
        writeln!(trfx, "082 0")?;
        writeln!(trfx, "092 IndividualDutch FIDE (JaVaFo)")?;
        writeln!(trfx, "102 IA Tournament Director")?;  // Set a default arbiter name
        writeln!(trfx, "112 Tournament Director")?;     // Set the same name for deputy arbiter
        writeln!(trfx, "122 300+3")?;                  // Standard format for time control
        writeln!(trfx, "XXR {}", self.rounds)?;
        
        // Add piece color configuration based on tournament ID
        let first_char = self.nanoid.chars().next().unwrap_or('0');
        let is_even = first_char.to_digit(10).unwrap_or(0) % 2 == 0;
        writeln!(trfx, "XXC {}1", if is_even { "white" } else { "black" })?;

        // Process each player
        for (i, player) in players.iter().enumerate() {
            let player_number = i + 1;
            let username = &player.username;
            let games_for_player = games.iter().filter(|g| g.white_id == player.id || g.black_id == player.id);
            let mut game_results = String::new();

            // Add game results if any
            for game in games_for_player {
                let opponent = if game.white_id == player.id {
                    game.black_id
                } else {
                    game.white_id
                };
                // Get opponent number from initial_seeding
                let opponent_number = self.initial_seeding
                    .iter()
                    .position(|&id| id == Some(opponent))
                    .map(|pos| pos + 1)
                    .unwrap_or(players.iter().position(|p| p.id == opponent).unwrap() + 1);
                let color = if game.white_id == player.id { "w" } else { "b" };
                let result = match game.tournament_game_result.as_str() {
                    "WhiteWin" if game.white_id == player.id => "1",
                    "BlackWin" if game.black_id == player.id => "1",
                    "Draw" => "=",
                    _ => "0",
                };
                game_results.push_str(&format!(" {:>4} {} {} ", opponent_number, color, result));
            }

            // Format player line according to FIDE standard:
            // 001 [4] [1] [3] [33] [4] [3] [11] [10] [4] [4]
            let player_line = format!(
                "{} {:>4} {} {:3} {:<33}{:>4} {:<3} {:>11} {:>10} {:>4} {:>4}",
                "001",             // 1-3: DataIdentificationnumber 001 (for player-data)
                player_number,     // 5-8: Starting rank (4 chars, right-aligned)
                "m",               // 10: Sex (1 char)
                "   ",             // 11-13: Title (3 chars, spaces if none)
                username,          // 15-47: Name (33 chars, left-aligned)
                0,                 // 49-52: FIDE Rating (4 chars, right-aligned)
                "   ",             // 54-56: FIDE Federation (3 chars, spaces if none)
                "0",               // 58-68: FIDE Number (11 chars, right-aligned)
                "0000/00/00",      // 70-79: Birth Date (10 chars, YYYY/MM/DD format)
                "0.0",             // 81-84: Current Points (4 chars with decimal)
                player_number      // 86-89: Rank (4 chars, right-aligned)
            );
            writeln!(trfx, "{}", player_line)?;
        }

        // Add XXA section for accelerated pairing
        if self.seeding_mode == "Accelerated" {
            println!("Adding XXA section for accelerated pairing...");
            let total_players = players.len();
            let top_half = (total_players + 1) / 2; // Round up for odd number of players
            
            for (i, player) in players.iter().enumerate() {
                let player_number = self.initial_seeding
                    .iter()
                    .position(|&id| id == Some(player.id))
                    .map(|pos| pos + 1)
                    .unwrap_or(i + 1);
                
                // Assign 1.0 points to top half, 0.0 to bottom half
                let fictitious_points = if player_number <= top_half { 1.0 } else { 0.0 };
                println!("Player {} gets fictitious points: {}", player_number, fictitious_points);
                writeln!(trfx, "XXA {:4}  {:3.1}", player_number, fictitious_points)?;
            }
        }

        println!("TRFx generation complete");
        Ok(trfx)
    }

    pub async fn save_trfx(&self, conn: &mut DbConn<'_>) -> Result<String, DbError> {
        let trfx_content = self.generate_trfx(conn).await?;
        
        // Generate filename using tournament nanoid
        let now = Utc::now();
        let trfx_filename = format!(
            "{}_{}_{}_{}_round_{}.trfx",
            now.format("%Y"),
            now.format("%m"),
            now.format("%d"),
            self.nanoid,
            self.current_round
        );
        
        // Save TRFx file with relative path
        let trfx_dir = "/Users/leex/src/hive/trfx";
        let trfx_path = format!("{}/{}", trfx_dir, trfx_filename);
        println!("TRFx file saved as: {}", trfx_path);
        
        // Create trfx directory if it doesn't exist
        std::fs::create_dir_all(&trfx_dir).map_err(|e| {
            DbError::InvalidInput {
                info: "Failed to create trfx directory".to_string(),
                error: e.to_string(),
            }
        })?;
        
        std::fs::write(&trfx_path, &trfx_content).map_err(|e| {
            DbError::InvalidInput {
                info: "Failed to save TRFx file".to_string(),
                error: e.to_string(),
            }
        })?;
        println!("TRFx file saved to: {}", trfx_path);

        // Save pairing output with relative path
        let pairing_path = format!("{}/{}_pairing", trfx_dir, trfx_filename.trim_end_matches(".trfx"));
        println!("Pairing output will be saved to: {}", pairing_path);
        
        Ok(trfx_path)
    }

    pub async fn create_games_from_pairing_file(&self, pairing_file_path: &str, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        println!("Reading pairing file: {}", pairing_file_path);
        
        // Read the pairing file
        let content = std::fs::read_to_string(pairing_file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        // First line contains the number of pairings
        let num_pairings = lines[0].parse::<usize>().map_err(|e| DbError::InvalidInput {
            info: String::from("Failed to parse number of pairings"),
            error: e.to_string(),
        })?;
        
        println!("Found {} pairings", num_pairings);
        let games_per_round = self.games_per_round;
        println!("Games per round: {}", games_per_round);
        
        let mut games = Vec::new();
        
        // Process each pairing line
        for (i, line) in lines[1..=num_pairings].iter().enumerate() {
            let pair: Vec<&str> = line.split_whitespace().collect();
            if pair.len() != 2 {
                return Err(DbError::InvalidInput {
                    info: format!("Invalid pairing format at line {}", i + 2),
                    error: String::from("Each line must contain exactly two numbers"),
                });
            }
            
            // Parse player numbers
            let white_number = pair[0].parse::<usize>().map_err(|e| DbError::InvalidInput {
                info: format!("Failed to parse white player number at line {}", i + 2),
                error: e.to_string(),
            })?;
            
            let black_number = pair[1].parse::<usize>().map_err(|e| DbError::InvalidInput {
                info: format!("Failed to parse black player number at line {}", i + 2),
                error: e.to_string(),
            })?;
            
            // Skip entirely if this is a BYE (black_number = 0)
            if black_number == 0 {
                println!("Player {} has a BYE - skipping game creation", white_number);
                continue;
            }
            
            // Get player IDs from initial_seeding
            let white_id = self.initial_seeding.get(white_number - 1)
                .and_then(|id| *id)
                .ok_or_else(|| DbError::InvalidInput {
                    info: format!("Invalid white player number: {}", white_number),
                    error: format!("Player number {} not found in initial seeding (array length: {})", white_number, self.initial_seeding.len()),
                })?;
            
            let black_id = self.initial_seeding.get(black_number - 1)
                .and_then(|id| *id)
                .ok_or_else(|| DbError::InvalidInput {
                    info: format!("Invalid black player number: {}", black_number),
                    error: format!("Player number {} not found in initial seeding (array length: {})", black_number, self.initial_seeding.len()),
                })?;
            
            println!("Creating {} games for pairing: {} (white) vs {} (black)", 
                games_per_round, white_number, black_number);
            
            // Create games_per_round games for this pairing
            for game_num in 0..games_per_round {
                // For the first game, use the colors from the pairing file
                // For subsequent games, alternate colors if game_num is odd
                let (game_white_id, game_black_id) = if game_num % 2 == 0 {
                    (white_id, black_id)
                } else {
                    (black_id, white_id)
                };
                
                println!("Creating game {}/{}: {} (white) vs {} (black)", 
                    game_num + 1, games_per_round, 
                    if game_num % 2 == 0 { white_number } else { black_number },
                    if game_num % 2 == 0 { black_number } else { white_number });
                
                // Create the game
                let new_game = NewGame::new_from_tournament(game_white_id, game_black_id, self);
                let game = Game::create(new_game, conn).await?;
                games.push(game);
            }
        }
        
        println!("Created {} games from pairing file", games.len());
        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::NewUser;
    use crate::models::user::User;
    use diesel_async::AsyncConnection;
    use shared_types::{ScoringMode, SeedingMode, StartMode, Tiebreaker, TimeMode};

    /// Create a test Swiss tournament with the specified number of players
    async fn create_test_swiss_tournament(
        conn: &mut DbConn<'_>,
        num_players: usize,
        games_per_round: i32,
        seeding_mode: SeedingMode,
    ) -> Result<(Tournament, Vec<User>), DbError> {
        println!("TEST: Creating Swiss tournament with {} players, {} games per round, seeding: {:?}", 
                 num_players, games_per_round, seeding_mode);
        
        // Create a test tournament
        let details = TournamentDetails {
            name: format!("Test Swiss Tournament {}", nanoid!(5)),
            description: "Test tournament for Swiss pairing".to_string(),
            scoring: ScoringMode::Game,
            tiebreakers: vec![Some(Tiebreaker::HeadToHead)],
            invitees: Vec::new(),
            seats: num_players as i32,
            min_seats: 2,
            rounds: 3,
            invite_only: false,
            mode: "Swiss".to_string(),
            time_mode: TimeMode::RealTime,
            time_base: Some(300),
            time_increment: Some(3),
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual,
            starts_at: None,
            round_duration: None,
            series: None,
            seeding_mode: Some(seeding_mode),
            games_per_round,
        };

        println!("TEST: Creating tournament with details: {:?}", details);
        let new_tournament = NewTournament::new(details)?;
        println!("TEST: NewTournament created successfully");
        
        // Create organizer user
        let organizer_details = NewUser {
            username: format!("organizer_{}", nanoid!(5)),
            password: "hash".to_string(),
            email: format!("organizer_{}@example.com", nanoid!(5)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            normalized_username: format!("organizer_{}", nanoid!(5)).to_lowercase(),
            patreon: false,
        };
        println!("TEST: Creating organizer with username: {}", organizer_details.username);
        let organizer = User::create(organizer_details, conn).await?;
        println!("TEST: Organizer created with ID: {}", organizer.id);
        
        // Create tournament
        println!("TEST: Creating tournament with organizer ID: {}", organizer.id);
        let tournament = Tournament::create(organizer.id, &new_tournament, conn).await?;
        println!("TEST: Tournament created with ID: {}", tournament.id);
        
        // Create players and make them join
        let mut players = Vec::new();
        for i in 0..num_players {
            let player_details = NewUser {
                username: format!("player_{}", i),
                password: "hash".to_string(),
                email: format!("player_{}@example.com", i),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                normalized_username: format!("player_{}", i).to_lowercase(),
                patreon: false,
            };
            println!("TEST: Creating player {} with username: {}", i, player_details.username);
            let player = User::create(player_details, conn).await?;
            println!("TEST: Player created with ID: {}", player.id);
            
            // Create a fake rating entry for this player
            let rating_value = 1500 + (i as i32 * 10); // Spread ratings for proper seeding
            let deviation = 50;
            println!("TEST: Creating rating for player {}: rating={}, deviation={}", 
                     player.id, rating_value, deviation);
            let result = diesel::sql_query(format!(
                "INSERT INTO ratings (user_uid, rating, deviation, speed, created_at, updated_at) VALUES ('{}', {}, {}, '{}', current_timestamp, current_timestamp)",
                player.id, rating_value, deviation, "RealTime"
            ))
            .execute(conn)
            .await;
            
            if let Err(ref e) = result {
                println!("TEST ERROR: Failed to insert rating for player {}: {:?}", player.id, e);
                return Err(DbError::InternalError);
            }
            println!("TEST: Rating created for player");
            
            // Join tournament
            println!("TEST: Player {} joining tournament {}", player.id, tournament.id);
            tournament.join(&player.id, conn).await?;
            println!("TEST: Player joined tournament successfully");
            players.push(player);
        }
        
        println!("TEST: Successfully created tournament with {} players", players.len());
        Ok((tournament, players))
    }

    #[tokio::test]
    async fn test_swiss_tournament_with_even_players() {
        println!("\n\n=== STARTING TEST: test_swiss_tournament_with_even_players ===");
        
        // Use the common test database connection pool
        println!("TEST: Getting database pool");
        let pool = crate::test_utils::get_pool();
        
        // Add retry logic for getting a connection
        let mut conn = loop {
            match pool.get().await {
                Ok(conn) => {
                    println!("TEST: Connection obtained successfully");
                    break conn;
                }
                Err(e) => {
                    println!("TEST WARNING: Failed to get connection, retrying: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        };
        
        // Start a test transaction that will be rolled back automatically
        println!("TEST: Starting test transaction");
        match conn.begin_test_transaction().await {
            Ok(_) => println!("TEST: Test transaction started successfully"),
            Err(e) => {
                println!("TEST ERROR: Failed to start test transaction: {:?}", e);
                panic!("Failed to start test transaction: {:?}", e);
            }
        }
        
        // Check if pairer executable exists
        let pairer_path = if cfg!(target_os = "macos") {
            "/Users/leex/src/hive/tools/macos/pairer"
        } else {
            "/Users/leex/src/hive/tools/linux/pairer"
        };
        
        if !std::path::Path::new(pairer_path).exists() {
            panic!("Pairer executable not found at {}. Test cannot proceed.", pairer_path);
        }
        println!("TEST: Found pairer executable at: {}", pairer_path);
        
        // Create a tournament with 8 players and 1 game per round (standard seeding)
        println!("TEST: Creating test tournament with 8 players");
        let tournament_result = create_test_swiss_tournament(&mut conn, 8, 1, SeedingMode::Standard).await;
        
        match tournament_result {
            Ok((tournament, _players)) => {
                println!("TEST: Test tournament created successfully with ID: {}", tournament.id);
                
                // Call swiss_start to generate pairings and create games
                println!("TEST: Starting Swiss tournament");
                match tournament.swiss_start(&mut conn).await {
                    Ok(games) => {
                        println!("TEST: Swiss tournament started successfully, created {} games", games.len());
                        
                        // Verify the number of games created is correct
                        println!("TEST: Verifying game count: expected 4, actual {}", games.len());
                        assert_eq!(games.len(), 4, "Expected 4 games for 8 players with 1 game per round");
                    }
                    Err(e) => {
                        println!("TEST ERROR: Failed to start Swiss tournament: {:?}", e);
                        panic!("Failed to start Swiss tournament: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("TEST ERROR: Failed to create test tournament: {:?}", e);
                panic!("Failed to create test tournament: {:?}", e);
            }
        }
        
        // Clean up test files
        println!("TEST: Cleaning up test files");
        let trfx_dir = "/Users/leex/src/hive/trfx";
        if let Ok(entries) = std::fs::read_dir(&trfx_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && (path.to_string_lossy().ends_with(".trfx") || path.to_string_lossy().contains("_pairing")) {
                        println!("TEST: Removing file: {:?}", path);
                        if let Err(e) = std::fs::remove_file(path) {
                            println!("TEST WARNING: Failed to remove file: {:?}", e);
                        }
                    }
                }
            }
        } else {
            println!("TEST WARNING: Failed to read trfx directory");
        }
        
        println!("=== TEST COMPLETED: test_swiss_tournament_with_even_players ===\n\n");
    }

    #[tokio::test]
    async fn test_swiss_tournament_with_odd_players() {
        println!("\n\n=== STARTING TEST: test_swiss_tournament_with_odd_players ===");
        
        // Use the common test database connection pool
        println!("TEST: Getting database pool");
        let pool = crate::test_utils::get_pool();
        println!("TEST: Pool obtained, getting connection");
        let mut conn = pool.get().await.expect("Failed to get connection");
        println!("TEST: Connection obtained");
        
        // Start a test transaction that will be rolled back automatically
        println!("TEST: Starting test transaction");
        conn.begin_test_transaction().await.expect("Failed to start test transaction");
        println!("TEST: Test transaction started");
        
        // Check if pairer executable exists
        let pairer_path = if cfg!(target_os = "macos") {
            "/Users/leex/src/hive/tools/macos/pairer"
        } else {
            "/Users/leex/src/hive/tools/linux/pairer"
        };
        
        if !std::path::Path::new(pairer_path).exists() {
            panic!("Pairer executable not found at {}. Test cannot proceed.", pairer_path);
        }
        println!("TEST: Found pairer executable at: {}", pairer_path);
        
        // Create a tournament with 13 players and 2 games per round (accelerated seeding)
        println!("TEST: Creating test tournament with 13 players");
        let tournament_result = create_test_swiss_tournament(&mut conn, 13, 2, SeedingMode::Accelerated).await;
        
        if let Err(ref e) = tournament_result {
            println!("TEST ERROR: Failed to create test tournament: {:?}", e);
            panic!("Failed to create test tournament: {:?}", e);
        }
        
        let (tournament, _players) = tournament_result.expect("Failed to create test tournament");
        println!("TEST: Test tournament created successfully with ID: {}", tournament.id);
        
        // Call swiss_start to generate pairings and create games
        println!("TEST: Starting Swiss tournament");
        let start_result = tournament.swiss_start(&mut conn).await;
        
        if let Err(ref e) = start_result {
            println!("TEST ERROR: Failed to start Swiss tournament: {:?}", e);
            panic!("Failed to start Swiss tournament: {:?}", e);
        }
        
        let games = start_result.expect("Failed to start Swiss tournament");
        println!("TEST: Swiss tournament started successfully, created {} games", games.len());
        
        // Verify the number of games created is correct: 3 pairings * 2 games per round = 6 games for 7 players
        // With 13 players, we expect 6 pairings * 2 games per round = 12 games (and one player gets a bye)
        println!("TEST: Verifying game count: expected 12, actual {}", games.len());
        assert_eq!(games.len(), 12, "Expected 12 games for 13 players with 2 games per round");
        
        // Clean up test files
        println!("TEST: Cleaning up test files");
        let trfx_dir = "/Users/leex/src/hive/trfx";
        if let Ok(entries) = std::fs::read_dir(&trfx_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && (path.to_string_lossy().ends_with(".trfx") || path.to_string_lossy().contains("_pairing")) {
                        println!("TEST: Removing file: {:?}", path);
                        if let Err(e) = std::fs::remove_file(path) {
                            println!("TEST WARNING: Failed to remove file: {:?}", e);
                        }
                    }
                }
            }
        } else {
            println!("TEST WARNING: Failed to read trfx directory");
        }
        
        println!("=== TEST COMPLETED: test_swiss_tournament_with_odd_players ===\n\n");
    }
}
