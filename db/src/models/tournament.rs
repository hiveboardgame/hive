// TODO: FIX BYE generation and rounds

use super::{Game, NewGame, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        rating::Rating, tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser,
        user::User,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        tournaments::{
            self, current_round, ends_at, nanoid as nanoid_field, series as series_column,
            started_at, starts_at, status as status_column, updated_at,
        },
        tournaments_organizers, users,
    },
    DbConn,
};
use chrono::Utc;
use chrono::{prelude::*, TimeDelta};
use diesel::prelude::*;
use diesel::BelongingToDsl;
use diesel_async::RunQueryDsl;
use hive_lib::Color;
use itertools::Itertools;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{
    Certainty, GameSpeed, Standings, Tiebreaker, TimeMode,
    TournamentDetails, TournamentGameResult, TournamentId, TournamentSortOrder, TournamentStatus,
};
use std::fmt::Write;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
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
    pub games_per_round: i32,
    pub bye: Vec<Option<Uuid>>,
    pub current_round: i32,
    pub initial_seeding: Vec<Option<Uuid>>,
    pub accelerated_rounds: i32,
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
            games_per_round: details.games_per_round,
            accelerated_rounds: details.accelerated_rounds,
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
    pub games_per_round: i32,
    pub bye: Vec<Option<Uuid>>,
    pub current_round: i32,
    pub initial_seeding: Vec<Option<Uuid>>,
    pub accelerated_rounds: i32,
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
        println!(
            "Setting up initial seeding for Swiss tournament with {} players",
            players.len()
        );

        // Create a mapping of GameSpeed from tournament time settings
        let game_speed = GameSpeed::from_base_increment(self.time_base, self.time_increment);
        println!("Tournament game speed: {}", game_speed);

        // Get player ratings and information
        let mut player_ratings = Vec::new();
        for player in &players {
            let rating = Rating::for_uuid(&player.id, &game_speed, conn).await?;

            // Determine certainty level from deviation
            let certainty = Certainty::from_deviation(rating.deviation);

            println!(
                "Player {}: rating={}, deviation={}, certainty={:?}",
                player.username, rating.rating, rating.deviation, &certainty
            );

            player_ratings.push((player.id, rating.rating as u64, certainty));
        }

        // Sort players: first by certainty level, then by rating
        player_ratings.sort_by(|a, b| {
            // First sort by certainty (using its Ord implementation)
            match a.2.cmp(&b.2) {
                std::cmp::Ordering::Equal => {
                    // If certainty is equal, sort by rating (higher first)
                    b.1.cmp(&a.1)
                }
                other_ordering => other_ordering,
            }
        });

        // Create the initial seeding array
        let initial_seeding: Vec<Option<Uuid>> = player_ratings
            .iter()
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

    pub async fn swiss_start_round(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Self, Vec<Game>), DbError> {
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
        let games = tournament
            .create_games_from_pairing_file(&pairings_file_path, conn)
            .await?;
        println!("Created {} games from pairings", games.len());

        // 5. Update tournament to indicate first round is created
        let tournament = diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Utc::now()),
                current_round.eq(self.current_round + 1),
            ))
            .get_result(conn)
            .await?;

        Ok((tournament, games))
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
        println!(
            "Executing pairer: {} --dutch {} -p {}",
            pairer_path, trfx_file_path, output_file_path
        );
        let output = std::process::Command::new(pairer_path)
            .arg("--dutch")
            .arg(trfx_file_path)
            .arg("-p")
            .arg(&output_file_path)
            .output()
            .map_err(|e| DbError::InvalidInput {
                info: "Failed to execute pairer".to_string(),
                error: e.to_string(),
            })?;

        // Print the error output if any
        if !output.stderr.is_empty() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            println!("Pairer error output: {}", error_msg);
        }

        if !output.status.success() {
            return Err(DbError::InvalidInput {
                info: "Pairing program failed".to_string(),
                error: format!(
                    "Exit code: {:?}\nError: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        // Verify the pairing file was created
        if !std::path::Path::new(&output_file_path).exists() {
            return Err(DbError::InvalidInput {
                info: "Pairing file was not created".to_string(),
                error: "Pairer completed but output file does not exist".to_string(),
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

    pub async fn calculate_standings(&self, round_number: Option<u32>, conn: &mut DbConn<'_>) -> Result<Standings, DbError> {
        let mut standings = Standings::new();

        // Add tiebreakers from tournament settings
        for tiebreaker in self.tiebreaker.iter().flatten() {
            if let Ok(tb) = Tiebreaker::from_str(tiebreaker) {
                standings.add_tiebreaker(tb);
            }
        }

        // Get all games for this tournament
        let mut games = self.games(conn).await?;
        
        // Filter games by round if round_number is provided
        if let Some(round) = round_number {
            games.retain(|g| g.round.is_some_and(|r| r < round as i32));
        }

        // Add all players to standings
        let players = self.players(conn).await?;
        for player in &players {
            standings.players.insert(player.id);
        }

        // Add results from games
        println!("Adding results for {} games", games.len());
        for game in games {
            let result = TournamentGameResult::from_str(&game.tournament_game_result)
                .expect("TGR should be correctly set");
            standings.add_result(
                game.white_id,
                game.black_id,
                game.white_rating.unwrap_or(1500.0),
                game.black_rating.unwrap_or(1500.0),
                result,
            );
        }

        // Add byes if any
        for player_id in self.bye.iter().flatten() {
            // Add bye points for each game in the round
            for _ in 0..self.games_per_round {
                standings.add_bye(*player_id);
            }
        }

        standings.enforce_tiebreakers();
        Ok(standings)
    }

    pub async fn generate_trfx(
        &self,
        round_number: u32,
        conn: &mut DbConn<'_>,
    ) -> Result<String, DbError> {
        let mut trfx = String::new();
        println!("Starting TRFx generation for tournament: {}", self.name);
        println!(
            "Tournament mode: accelerated_rounds={}",
            self.accelerated_rounds
        );

        let players = self.players(conn).await?;
        let games = self.games(conn).await?;
        println!("Found {} games", games.len());

        // Build header section
        writeln!(trfx, "012 {}", self.name)?;
        writeln!(trfx, "022 Hivegame.com")?;
        writeln!(trfx, "032 Hiveystan")?;
        writeln!(trfx, "042 {}", Utc::now().format("%Y-%m-%d"))?;
        writeln!(trfx, "052 {}", Utc::now().format("%Y-%m-%d"))?;
        writeln!(trfx, "062 {}", players.len())?;
        writeln!(trfx, "072 0")?;
        writeln!(trfx, "082 0")?;
        writeln!(trfx, "092 IndividualDutch FIDE (JaVaFo)")?;
        writeln!(trfx, "102 IA Tournament Director")?; // Set a default arbiter name
        writeln!(trfx, "112 Tournament Director")?; // Set the same name for deputy arbiter
        writeln!(trfx, "122 300+3")?; // Standard format for time control
        writeln!(trfx, "XXR {}", self.rounds * self.games_per_round)?;

        // Add piece color configuration based on tournament ID
        let first_char = self.nanoid.chars().next().unwrap_or('0');
        let is_even = first_char.to_digit(10).unwrap_or(0) % 2 == 0;
        writeln!(trfx, "XXC {}1", if is_even { "white" } else { "black" })?;

        // Calculate standings
        let standings = self.calculate_standings(Some(round_number), conn).await?;

        for (player_number, player_id) in self.initial_seeding.iter().enumerate() {
            let player_id = player_id.expect("There should not be Nones in the initial_seeding");
            let player_number = player_number + 1;

            println!("There's {} game total", games.len());
            let player_games: Vec<_> = games
                .iter()
                .filter(|g| {
                    g.round <= Some(round_number as i32 - 1)
                        && (g.white_id == player_id || g.black_id == player_id)
                })
                .sorted_by_key(|g| (g.round, g.id))
                .collect();

            println!(
                "{} Games found for player: {:?}",
                player_games.len(),
                player_games
            );

            // This needs to iter over 0..current_round then find the games for the player of that
            // round if it cannot find any games for the current round it needs to check if the
            // player_id is in self.bye and then we need to add "0000 - U" as below
            let mut round_results = String::new();
            for i in 0..self.current_round {
                let round_games: Vec<_> = games
                    .iter()
                    .filter(|g| {
                        g.round == Some(i) && (g.white_id == player_id || g.black_id == player_id)
                    })
                    .sorted_by_key(|g| (g.round, g.id))
                    .collect();
                for game in round_games.iter() {
                    if round_results.is_empty() {
                        round_results.push_str("  ");
                    } else {
                        round_results.push_str("   ");
                    }
                    let opponent_number = if game.white_id == player_id {
                        self.initial_seeding
                            .iter()
                            .position(|&id| id == Some(game.black_id))
                            .map(|pos| pos + 1)
                            .unwrap_or(0)
                    } else {
                        self.initial_seeding
                            .iter()
                            .position(|&id| id == Some(game.white_id))
                            .map(|pos| pos + 1)
                            .unwrap_or(0)
                    };
                    let color = if game.white_id == player_id { "w" } else { "b" };
                    let result = match TournamentGameResult::from_str(&game.tournament_game_result)
                        .expect("TGR should be correctly set")
                    {
                        TournamentGameResult::Winner(color) => {
                            if game.white_id == player_id {
                                if color == Color::White {
                                    "1"
                                } else {
                                    "0"
                                }
                            } else if color == Color::White {
                                "0"
                            } else {
                                "1"
                            }
                        }
                        TournamentGameResult::Draw => "=",
                        TournamentGameResult::Unknown => "-",
                        TournamentGameResult::DoubeForfeit => "0",
                        TournamentGameResult::Bye => "U",
                    };
                    round_results.push_str(&format!("{:>3} {} {}", opponent_number, color, result));
                }
                if round_games.is_empty() {
                    if self.bye.get(i as usize) != Some(&Some(player_id)) {
                        return Err(DbError::InvalidInput {
                            info: "Couldn't find result but player is not bye".to_string(),
                            error: "Tournament has invalid state".to_string(),
                        });
                    };
                    for _ in 0..self.games_per_round {
                        if !round_results.is_empty() {
                            round_results.push(' ');
                        }
                        round_results.push_str(" 0000 - U");
                    }
                }
            }

            // Get player's current score from standings
            println!("Scores: {:?}", standings.players_scores);
            let score = if let Some(scores) = standings.players_scores.get(&player_id) {
                scores.get(&Tiebreaker::RawPoints).unwrap_or(&0.0)
            } else {
                &0.0
            };

            // Format player line
            let player = players
                .iter()
                .find(|p| p.id == player_id)
                .expect("User to be preset in players");
            let username = format!("{:<33}", player.username);
            let player_line = format!(
                "{} {:>4} {} {:3} {}{:>4} {:<3} {:>11} {:>10} {:>4} {:>4} {}",
                "001",                   // 1-3: DataIdentificationnumber 001 (for player-data)
                player_number,           // 5-8: Starting rank (4 chars, right-aligned)
                "?",                     // 10: Sex (1 char)
                "   ",                   // 11-13: Title (3 chars, spaces if none)
                username,                // 15-47: Name (33 chars, left-aligned)
                0,                       // 49-52: FIjDE Rating (4 chars, right-aligned)
                "   ",                   // 54-56: FIDE Federation (3 chars, spaces if none)
                "0",                     // 58-68: FIDE Number (11 chars, right-aligned)
                "0000/00/00",            // 70-79: Birth Date (10 chars, YYYY/MM/DD format)
                format!("{:.1}", score), // 81-84: Current Points (4 chars with decimal)
                player_number,           // 86-89: Rank (4 chars, right-aligned)
                round_results            // 91+: Round-by-round results
            );
            writeln!(trfx, "{}", player_line)?;
        }

        // Add XXA section for accelerated pairing
        if self.accelerated_rounds > round_number as i32 {
            println!("Adding XXA section for accelerated pairing...");
            let total_players = players.len();
            let top_half = total_players.div_ceil(2);

            for (i, player) in players.iter().enumerate() {
                let player_number = self
                    .initial_seeding
                    .iter()
                    .position(|&id| id == Some(player.id))
                    .map(|pos| pos + 1)
                    .unwrap_or(i + 1);

                // Assign 1.0 points to top half, 0.0 to bottom half
                let fictitious_points = if player_number <= top_half { 1.0 } else { 0.0 };
                println!(
                    "Player {} gets fictitious points: {}",
                    player_number, fictitious_points
                );
                writeln!(trfx, "XXA {:4}  {:3.1}", player_number, fictitious_points)?;
            }
        }

        println!("TRFx generation complete");
        Ok(trfx)
    }

    pub async fn save_trfx(&self, conn: &mut DbConn<'_>) -> Result<String, DbError> {
        let trfx_content = self.generate_trfx(self.current_round as u32, conn).await?;

        // Generate filename using tournament nanoid
        let now = Utc::now();
        println!("############################################################");
        println!("Current round is: {}", self.current_round);
        println!("############################################################");
        let trfx_filename = format!(
            "{}_{}_{}_{}_round_{}.trfx",
            now.format("%Y"),
            now.format("%m"),
            now.format("%d"),
            self.nanoid,
            self.current_round + 1
        );

        // Save TRFx file with relative path
        let trfx_dir = "/Users/leex/src/hive/trfx";
        let trfx_path = format!("{}/{}", trfx_dir, trfx_filename);
        println!("TRFx file saved as: {}", trfx_path);

        // Create trfx directory if it doesn't exist
        std::fs::create_dir_all(trfx_dir).map_err(|e| DbError::InvalidInput {
            info: "Failed to create trfx directory".to_string(),
            error: e.to_string(),
        })?;

        std::fs::write(&trfx_path, &trfx_content).map_err(|e| DbError::InvalidInput {
            info: "Failed to save TRFx file".to_string(),
            error: e.to_string(),
        })?;
        println!("TRFx file saved to: {}", trfx_path);

        // Save pairing output with relative path
        let pairing_path = format!(
            "{}/{}_pairing",
            trfx_dir,
            trfx_filename.trim_end_matches(".trfx")
        );
        println!("Pairing output will be saved to: {}", pairing_path);

        Ok(trfx_path)
    }

    pub async fn create_games_from_pairing_file(
        &self,
        pairing_file_path: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let mut games = Vec::new();
        let pairing_content =
            std::fs::read_to_string(pairing_file_path).map_err(|e| DbError::InvalidInput {
                info: "Failed to read pairing file".to_string(),
                error: e.to_string(),
            })?;

        // Skip the first line which contains the number of pairings
        for line in pairing_content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let white_number = parts[0].parse::<i32>().map_err(|e| DbError::InvalidInput {
                    info: "Failed to parse white player number".to_string(),
                    error: e.to_string(),
                })?;
                let black_number = parts[1].parse::<i32>().map_err(|e| DbError::InvalidInput {
                    info: "Failed to parse black player number".to_string(),
                    error: e.to_string(),
                })?;

                // Handle bye (black_number is 0)
                if black_number == 0 {
                    // Get the player who got the bye
                    let bye_player_uuid = self.initial_seeding[white_number as usize - 1]
                        .ok_or_else(|| DbError::InvalidInput {
                            info: format!("No player found for white number {}", white_number),
                            error: "Invalid player number".to_string(),
                        })?;

                    // Add the player to the bye array
                    let mut updated_bye = self.bye.clone();
                    updated_bye.push(Some(bye_player_uuid));

                    // Update the tournament with the new bye array
                    diesel::update(self)
                        .set(tournaments::bye.eq(updated_bye))
                        .execute(conn)
                        .await?;
                    // Skip creating games for bye players
                    continue;
                } else {
                    // No bye player, add None to the bye array
                    let mut updated_bye = self.bye.clone();
                    updated_bye.push(None);

                    // Update the tournament with the new bye array
                    diesel::update(self)
                        .set(tournaments::bye.eq(updated_bye))
                        .execute(conn)
                        .await?;
                }

                // Get player UUIDs from their numbers
                let white_uuid =
                    self.initial_seeding[white_number as usize - 1].ok_or_else(|| {
                        DbError::InvalidInput {
                            info: format!("No player found for white number {}", white_number),
                            error: "Invalid player number".to_string(),
                        }
                    })?;
                let black_uuid =
                    self.initial_seeding[black_number as usize - 1].ok_or_else(|| {
                        DbError::InvalidInput {
                            info: format!("No player found for black number {}", black_number),
                            error: "Invalid player number".to_string(),
                        }
                    })?;

                // Create games_per_round number of games for this pairing
                for game_num in 0..self.games_per_round {
                    // Alternate colors based on game number
                    let (white_uuid, black_uuid) = if game_num % 2 == 0 {
                        (white_uuid, black_uuid)
                    } else {
                        (black_uuid, white_uuid)
                    };

                    let new_game = NewGame::new_from_tournament(white_uuid, black_uuid, self);
                    let game = Game::create(new_game, conn).await?;
                    games.push(game);
                }
            }
        }

        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rating::Rating;
    use crate::models::user::NewUser;
    use crate::models::user::User;
    use diesel_async::AsyncConnection;
    use hive_lib::Color;
    use hive_lib::GameControl;
    use shared_types::{GameSpeed, ScoringMode, StartMode, Tiebreaker, TimeMode};

    /// Create a test Swiss tournament with the specified number of players
    async fn create_test_swiss_tournament(
        conn: &mut DbConn<'_>,
        num_players: usize,
        games_per_round: i32,
        accelerated_rounds: i32,
    ) -> Result<(Tournament, Vec<User>), DbError> {
        println!("TEST: Creating Swiss tournament with {} players, {} games per round, accelerated rounds: {}", 
                 num_players, games_per_round, accelerated_rounds);

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
            accelerated_rounds,
            games_per_round,
        };

        println!("TEST: Creating tournament with details: {:?}", details);
        let new_tournament = NewTournament::new(details)?;
        println!("TEST: NewTournament created successfully");

        // Create organizer user
        let organizer_username = format!("organizer_{}", nanoid!(5));
        println!("TEST: Creating organizer with username: {}", organizer_username);
        let organizer = NewUser::new(
            &organizer_username,
            "test_password",
            "test@example.com",
        )?;
        let organizer = User::create(organizer, conn).await?;
        println!("TEST: Organizer created with ID: {}", organizer.id);

        // Create tournament with organizer
        println!("TEST: Creating tournament with organizer ID: {}", organizer.id);
        let tournament = Tournament::create(organizer.id, &new_tournament, conn).await?;
        println!("TEST: Tournament created with ID: {}", tournament.id);

        // Create players in a transaction
        let mut players = Vec::new();
        for i in 1..=num_players {
            let player_username = format!("player_{}", i);
            println!("TEST: Creating player {} with username: {}", i, player_username);
            let player = NewUser::new(
                &player_username,
                "test_password",
                &format!("player{}@example.com", i),
            )?;
            let player = User::create(player, conn).await?;
            println!("TEST: Player created with ID: {}", player.id);

            // Update the player's rating to the desired value
            let game_speed = GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
            let rating_value = 2000.0 - (i as f64 * 25.0); // Spread ratings for proper seeding
            let deviation = 50.0;
            println!(
                "TEST: Updating rating for player {}: rating={}, deviation={}, game_speed={}",
                player.id, rating_value, deviation, game_speed
            );

            diesel::update(crate::schema::ratings::table)
                .filter(crate::schema::ratings::user_uid.eq(player.id))
                .filter(crate::schema::ratings::speed.eq(game_speed.to_string()))
                .set((
                    crate::schema::ratings::rating.eq(rating_value),
                    crate::schema::ratings::deviation.eq(deviation),
                    crate::schema::ratings::updated_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;

            println!("TEST: Rating updated for player");

            tournament.join(&player.id, conn).await?;
            println!("TEST: Player joined tournament successfully");
            players.push(player);
        }

        println!(
            "TEST: Successfully created tournament with {} players",
            players.len()
        );
        Ok((tournament, players))
    }

    #[tokio::test]
    async fn test_swiss_tournament_with_even_players() -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n=== STARTING TEST: test_swiss_tournament_with_even_players ===");

        // Clean up pool from previous tests
        crate::test_utils::cleanup_pool().await;

        // Use the common test database connection pool
        println!("TEST: Getting database pool");
        let pool = crate::test_utils::get_pool();

        // Add retry logic for getting a connection with a maximum number of retries
        let mut retries = 0;
        const MAX_RETRIES: u32 = 5;
        let mut conn = loop {
            match pool.get().await {
                Ok(conn) => {
                    println!("TEST: Connection obtained successfully");
                    break conn;
                }
                Err(e) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        panic!("Failed to get connection after {} retries: {:?}", MAX_RETRIES, e);
                    }
                    println!(
                        "TEST WARNING: Failed to get connection (attempt {}/{}), retrying in 1s: {:?}",
                        retries, MAX_RETRIES, e
                    );
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
            panic!(
                "Pairer executable not found at {}. Test cannot proceed.",
                pairer_path
            );
        }
        println!("TEST: Found pairer executable at: {}", pairer_path);

        // Create a tournament with 8 players and 1 game per round (no acceleration)
        println!("TEST: Creating test tournament with 8 players");
        let tournament_result = create_test_swiss_tournament(&mut conn, 8, 1, 0).await;

        match tournament_result {
            Ok((tournament, _players)) => {
                println!(
                    "TEST: Test tournament created successfully with ID: {}",
                    tournament.id
                );

                // Round 1
                println!("\n=== ROUND 1 ===");
                println!("TEST: Starting Swiss tournament");
                match tournament.swiss_start_round(&mut conn).await {
                    Ok((_, games)) => {
                        println!(
                            "TEST: Swiss tournament started successfully, created {} games",
                            games.len()
                        );
                        println!(
                            "TEST: Verifying game count: expected 4, actual {}",
                            games.len()
                        );
                        assert_eq!(
                            games.len(),
                            4,
                            "Expected 4 games for 8 players with 1 game per round"
                        );

                        // Complete round 1 games
                        println!("TEST: Completing round 1 games");
                        for game in games {
                            // Get player ratings from the database
                            let game_speed = GameSpeed::from_base_increment(
                                tournament.time_base,
                                tournament.time_increment,
                            );
                            let white_rating =
                                Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;
                            let black_rating =
                                Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;

                            // Higher rated player wins
                            if white_rating >= black_rating {
                                game.resign(&GameControl::Resign(Color::Black), &mut conn)
                                    .await?;
                            } else {
                                game.resign(&GameControl::Resign(Color::White), &mut conn)
                                    .await?;
                            };
                        }
                    }
                    Err(e) => {
                        println!("TEST ERROR: Failed to start Swiss tournament: {:?}", e);
                        panic!("Failed to start Swiss tournament: {:?}", e);
                    }
                }

                // Round 2
                println!("\n=== ROUND 2 ===");
                println!("TEST: Starting round 2");
                match tournament.swiss_start_round(&mut conn).await {
                    Ok((_, games)) => {
                        println!(
                            "TEST: Round 2 started successfully, created {} games",
                            games.len()
                        );
                        println!(
                            "TEST: Verifying game count: expected 4, actual {}",
                            games.len()
                        );
                        assert_eq!(
                            games.len(),
                            4,
                            "Expected 4 games for 8 players with 1 game per round"
                        );

                        // Complete round 2 games
                        println!("TEST: Completing round 2 games");
                        for game in games {
                            // Get player ratings from the database
                            let game_speed =
                                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
                            let white_rating =
                                Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;
                            let black_rating =
                                Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;

                            if white_rating >= black_rating {
                                game.resign(&GameControl::Resign(Color::Black), &mut conn)
                                    .await?;
                            } else {
                                game.resign(&GameControl::Resign(Color::White), &mut conn)
                                    .await?;
                            };
                        }
                    }
                    Err(e) => {
                        println!("TEST ERROR: Failed to start round 2: {:?}", e);
                        panic!("Failed to start round 2: {:?}", e);
                    }
                }

                // Round 3
                println!("\n=== ROUND 3 ===");
                println!("TEST: Starting round 3");
                match tournament.swiss_start_round(&mut conn).await {
                    Ok((_, games)) => {
                        println!(
                            "TEST: Round 3 started successfully, created {} games",
                            games.len()
                        );
                        println!(
                            "TEST: Verifying game count: expected 4, actual {}",
                            games.len()
                        );
                        assert_eq!(
                            games.len(),
                            4,
                            "Expected 4 games for 8 players with 1 game per round"
                        );

                        // Complete round 3 games
                        println!("TEST: Completing round 3 games");
                        for game in games {
                            // Get player ratings from the database
                            let game_speed =
                                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
                            let white_rating =
                                Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;
                            let black_rating =
                                Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                                    .await?
                                    .rating;

                            // Higher rated player wins
                            if white_rating >= black_rating {
                                game.resign(&GameControl::Resign(Color::Black), &mut conn)
                                    .await?;
                            } else {
                                game.resign(&GameControl::Resign(Color::White), &mut conn)
                                    .await?;
                            };
                        }
                    }
                    Err(e) => {
                        println!("TEST ERROR: Failed to start round 3: {:?}", e);
                        panic!("Failed to start round 3: {:?}", e);
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
        if let Ok(entries) = std::fs::read_dir(trfx_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && (path.to_string_lossy().ends_with(".trfx")
                        || path.to_string_lossy().contains("_pairing"))
                {
                    println!("TEST: Removing file: {:?}", path);
                    if let Err(e) = std::fs::remove_file(path) {
                        println!("TEST WARNING: Failed to remove file: {:?}", e);
                    }
                }
            }
        } else {
            println!("TEST WARNING: Failed to read trfx directory");
        }

        println!("=== TEST COMPLETED: test_swiss_tournament_with_even_players ===\n\n");

        // Clean up pool after test
        crate::test_utils::cleanup_pool().await;

        Ok(())
    }

    #[tokio::test]
    async fn test_swiss_tournament_with_odd_players() -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n=== STARTING TEST: test_swiss_tournament_with_odd_players ===");

        // Clean up pool from previous tests
        crate::test_utils::cleanup_pool().await;

        // Use the common test database connection pool
        println!("TEST: Getting database pool");
        let pool = crate::test_utils::get_pool();
        println!("TEST: Pool obtained, getting connection");

        // Add retry logic for getting a connection with a maximum number of retries
        let mut retries = 0;
        const MAX_RETRIES: u32 = 5;
        let mut conn = loop {
            match pool.get().await {
                Ok(conn) => {
                    println!("TEST: Connection obtained successfully");
                    break conn;
                }
                Err(e) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        panic!("Failed to get connection after {} retries: {:?}", MAX_RETRIES, e);
                    }
                    println!(
                        "TEST WARNING: Failed to get connection (attempt {}/{}), retrying in 1s: {:?}",
                        retries, MAX_RETRIES, e
                    );
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
        };

        // Check if pairer executable exists
        let pairer_path = if cfg!(target_os = "macos") {
            "/Users/leex/src/hive/tools/macos/pairer"
        } else {
            "/Users/leex/src/hive/tools/linux/pairer"
        };

        if !std::path::Path::new(pairer_path).exists() {
            panic!(
                "Pairer executable not found at {}. Test cannot proceed.",
                pairer_path
            );
        }
        println!("TEST: Found pairer executable at: {}", pairer_path);

        // Create a tournament with 13 players and 2 games per round (accelerated seeding)
        println!("TEST: Creating test tournament with 13 players");
        let tournament_result = create_test_swiss_tournament(&mut conn, 13, 2, 1).await;

        if let Err(ref e) = tournament_result {
            println!("TEST ERROR: Failed to create test tournament: {:?}", e);
            panic!("Failed to create test tournament: {:?}", e);
        }

        let (tournament, _players) = tournament_result.expect("Failed to create test tournament");
        println!(
            "TEST: Test tournament created successfully with ID: {}",
            tournament.id
        );

        // Round 1
        println!("\n=== ROUND 1 ===");
        println!("TEST: Starting Swiss tournament");
        let start_result = tournament.swiss_start_round(&mut conn).await;

        if let Err(ref e) = start_result {
            println!("TEST ERROR: Failed to start Swiss tournament: {:?}", e);
            panic!("Failed to start Swiss tournament: {:?}", e);
        }

        let (tournament, games) = start_result.expect("Failed to start Swiss tournament");
        println!(
            "TEST: Swiss tournament started successfully, created {} games",
            games.len()
        );
        println!(
            "TEST: Verifying game count: expected 12, actual {}",
            games.len()
        );
        assert_eq!(
            games.len(),
            12,
            "Expected 12 games for 13 players with 2 games per round"
        );

        // Complete round 1 games
        println!("TEST: Completing round 1 games");
        for game in games {
            // Get player ratings from the database
            let game_speed =
                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
            let white_rating = Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                .await?
                .rating;
            let black_rating = Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                .await?
                .rating;

            // Higher rated player wins
            if white_rating >= black_rating {
                game.resign(&GameControl::Resign(Color::Black), &mut conn)
                    .await?;
            } else {
                game.resign(&GameControl::Resign(Color::White), &mut conn)
                    .await?;
            };
        }

        // Round 2
        println!("\n=== ROUND 2 ===");
        println!("TEST: Starting round 2");
        let round2_result = tournament.swiss_start_round(&mut conn).await;

        if let Err(ref e) = round2_result {
            println!("TEST ERROR: Failed to start round 2: {:?}", e);
            panic!("Failed to start round 2: {:?}", e);
        }

        let (tournament, games) = round2_result.expect("Failed to start round 2");
        println!(
            "TEST: Round 2 started successfully, created {} games",
            games.len()
        );
        println!(
            "TEST: Verifying game count: expected 12, actual {}",
            games.len()
        );
        assert_eq!(
            games.len(),
            12,
            "Expected 12 games for 13 players with 2 games per round"
        );

        // Complete round 2 games
        println!("TEST: Completing round 2 games");
        for game in games {
            // Get player ratings from the database
            let game_speed =
                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
            let white_rating = Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                .await?
                .rating;
            let black_rating = Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                .await?
                .rating;

            // Higher rated player wins
            let result = if white_rating >= black_rating {
                TournamentGameResult::Winner(Color::White).to_string()
            } else {
                TournamentGameResult::Winner(Color::Black).to_string()
            };
            diesel::update(&game)
                .set(games::tournament_game_result.eq(result))
                .execute(&mut conn)
                .await?;
        }

        // Round 3
        println!("\n=== ROUND 3 ===");
        println!("TEST: Starting round 3");
        let round3_result = tournament.swiss_start_round(&mut conn).await;

        if let Err(ref e) = round3_result {
            println!("TEST ERROR: Failed to start round 3: {:?}", e);
            panic!("Failed to start round 3: {:?}", e);
        }

        let (tournament, games) = round3_result.expect("Failed to start round 3");
        println!(
            "TEST: Round 3 started successfully, created {} games",
            games.len()
        );
        println!(
            "TEST: Verifying game count: expected 12, actual {}",
            games.len()
        );
        assert_eq!(
            games.len(),
            12,
            "Expected 12 games for 13 players with 2 games per round"
        );

        // Complete round 3 games
        println!("TEST: Completing round 3 games");
        for game in games {
            // Get player ratings from the database
            let game_speed =
                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
            let white_rating = Rating::for_uuid(&game.white_id, &game_speed, &mut conn)
                .await?
                .rating;
            let black_rating = Rating::for_uuid(&game.black_id, &game_speed, &mut conn)
                .await?
                .rating;

            // Higher rated player wins
            let result = if white_rating >= black_rating {
                TournamentGameResult::Winner(Color::White).to_string()
            } else {
                TournamentGameResult::Winner(Color::Black).to_string()
            };
            diesel::update(&game)
                .set(games::tournament_game_result.eq(result))
                .execute(&mut conn)
                .await?;
        }

        // Clean up test files
        println!("TEST: Cleaning up test files");
        let trfx_dir = "/Users/leex/src/hive/trfx";
        if let Ok(entries) = std::fs::read_dir(trfx_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && (path.to_string_lossy().ends_with(".trfx")
                        || path.to_string_lossy().contains("_pairing"))
                {
                    println!("TEST: Removing file: {:?}", path);
                    if let Err(e) = std::fs::remove_file(path) {
                        println!("TEST WARNING: Failed to remove file: {:?}", e);
                    }
                }
            }
        } else {
            println!("TEST WARNING: Failed to read trfx directory");
        }

        println!("=== TEST COMPLETED: test_swiss_tournament_with_odd_players ===\n\n");

        // Clean up pool after test
        crate::test_utils::cleanup_pool().await;

        Ok(())
    }
}
