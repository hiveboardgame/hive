#![feature(int_roundings)]
use super::{Game, NewGame, Rating, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        tournaments::{
            self, bye, current_round, nanoid as nanoid_field, series as series_column, started_at,
            starts_at, status as status_column, updated_at,
        },
        tournaments_organizers, users,
    },
    DbConn,
};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::Color;
use itertools::Itertools;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{
    GameSpeed, SeedingMode, Standings, Tiebreaker, TimeMode, TournamentDetails,
    TournamentGameResult, TournamentSortOrder, TournamentStatus, Pairing,
};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::str::FromStr;
use uuid::Uuid;

// Swiss tournament scoring constants
const WIN_SCORE: f64 = 1.0;
const DRAW_SCORE: f64 = 0.5;
const SCORE_COMPARISON_EPSILON: f64 = f64::EPSILON;

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

        // Verify all games belong to this tournament
        debug_assert!(
            games.iter().all(|g| g.tournament_id == Some(self.id)),
            "Found games not belonging to this tournament"
        );

        // Add tiebreakers from tournament configuration
        for tiebreaker in self.tiebreaker.iter().flatten() {
            standings.add_tiebreaker(Tiebreaker::from_str(tiebreaker).map_err(|e| {
                DbError::InvalidInput {
                    info: String::from("Invalid tiebreaker"),
                    error: e.to_string(),
                }
            })?);
        }

        // Verify tiebreakers were added
        debug_assert!(
            !standings.tiebreakers.is_empty(),
            "No tiebreakers added to standings"
        );

        // Add all games to standings
        for game in games.iter() {
            // Verify game participants are tournament players
            debug_assert!(
                players.iter().any(|p| p.id == game.white_id),
                "White player {} not in tournament",
                game.white_id
            );
            debug_assert!(
                players.iter().any(|p| p.id == game.black_id),
                "Black player {} not in tournament",
                game.black_id
            );

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

        // Handle byes from tournament.bye vector
        for (round, bye_player) in self.bye.iter().enumerate() {
            if let Some(player_id) = bye_player {
                // Add bye result for the player using our new dedicated method
                standings.add_bye_result(*player_id);
            }
        }

        // Calculate all tiebreakers
        standings.enforce_tiebreakers();

        // Verify all players have results
        debug_assert_eq!(
            standings.players_scores.len(),
            players.len(),
            "Not all players have scores in standings"
        );

        // Verify standings are complete
        debug_assert_eq!(
            standings.players_standings.iter().flatten().count(),
            players.len(),
            "Not all players appear in final standings"
        );

        Ok(standings)
    }

    pub async fn round_robin_start(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Self, Vec<Game>), DbError> {
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
        Ok((self.clone(), games))
    }

    pub async fn start(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        // Ensure tournament hasn't already started
        if self.is_started() {
            return Err(DbError::InvalidInput {
                info: String::from("Tournament has already started"),
                error: String::from("Cannot start a tournament that has already started"),
            });
        }

        // Get players
        let players = self.players(conn).await?;
        if players.is_empty() {
            return Err(DbError::InvalidInput {
                info: String::from("No players in tournament"),
                error: String::from("Cannot start a tournament with no players"),
            });
        }

        // Create games based on tournament mode
        let (mut tournament, games) = match self.mode.as_str() {
            "SWISS" => {
                let (t, g) = self.swiss_start(players.clone(), conn).await?;
                (t, g)
            }
            "RR" => {
                let (t, g) = self.round_robin_start(conn).await?;
                (t, g)
            }
            _ => {
                return Err(DbError::InvalidInput {
                    info: format!("Invalid tournament mode: {}", self.mode),
                    error: String::from("Tournament mode must be SWISS or RR"),
                })
            }
        };

        // Mark tournament as started and save
        tournament.status = TournamentStatus::InProgress.to_string();
        tournament.started_at = Some(Utc::now());
        diesel::update(tournaments::table.find(tournament.id))
            .set((
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Some(Utc::now())),
            ))
            .execute(conn)
            .await?;

        // Get list of player IDs to notify
        let player_ids: Vec<Uuid> = players.into_iter().map(|p| p.id).collect();

        Ok((tournament, games, player_ids))
    }

    pub async fn start_by_organizer(
        &self,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        self.ensure_user_is_organizer(organizer, conn).await?;
        self.start(conn).await
    }

    fn are_compatible_opponents(&self, p1: &PlayerState, p2: &PlayerState) -> bool {
        // Players cannot play against themselves
        if p1.player.id == p2.player.id {
            return false;
        }

        // Players cannot play against previous opponents
        if p1.has_played(p2.player.id) || p2.has_played(p1.player.id) {
            return false;
        }

        // Check if either player can play both colors
        p1.can_play_color(Color::White) && p2.can_play_color(Color::Black)
            || p1.can_play_color(Color::Black) && p2.can_play_color(Color::White)
    }

    fn determine_colors(&self, p1: &PlayerState, p2: &PlayerState) -> (Color, Color) {
        // If one player can only play one color, assign that
        if !p1.can_play_color(Color::Black) || !p2.can_play_color(Color::White) {
            return (Color::White, Color::Black);
        }
        if !p1.can_play_color(Color::White) || !p2.can_play_color(Color::Black) {
            return (Color::Black, Color::White);
        }

        // Calculate color scores for both possibilities
        let score1 = p1.color_score(Color::White) + p2.color_score(Color::Black);
        let score2 = p1.color_score(Color::Black) + p2.color_score(Color::White);

        // Choose the colors that maximize the combined score
        if score1 >= score2 {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::White)
        }
    }

    pub async fn find_bye_player_in_score_groups(
        &self,
        score_groups: &[Vec<PlayerState>],
        conn: &mut DbConn<'_>,
    ) -> Result<Option<Uuid>, DbError> {
        // Rule 8.1: A player may receive at most one bye in a tournament
        let players_with_byes: HashSet<Uuid> = self
            .bye
            .iter()
            .filter_map(|opt| opt.as_ref())
            .copied()
            .collect();

        // Start from the lowest score group
        for group in score_groups.iter().rev() {
            // Sort players within group by pairing number (rank)
            let mut group_players = group.clone();
            group_players.sort_by_key(|p| p.pairing_number);

            // Find the lowest ranked player who hasn't had a bye
            for player_state in group_players {
                if !players_with_byes.contains(&player_state.player.id) {
                    return Ok(Some(player_state.player.id));
                }
            }
        }
        Ok(None)
    }

    pub async fn find_bye_player(&self, conn: &mut DbConn<'_>) -> Result<Option<Uuid>, DbError> {
        // For first round, simply use the lowest rated player
        if self.current_round == 0 {
            let players_with_byes: HashSet<Uuid> = self
                .bye
                .iter()
                .filter_map(|opt| opt.as_ref())
                .copied()
                .collect();

            if let Ok(players) = self.players(conn).await {
                for player in players.iter().rev() {
                    // Reverse to start with lowest rated
                    if !players_with_byes.contains(&player.id) {
                        return Ok(Some(player.id));
                    }
                }
            }
        }
        Ok(None)
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
        let mut games = Vec::new();
        let players = self.players(conn).await?;
        let existing_games = self.games(conn).await?;

        // Verify all games from previous rounds are finished
        debug_assert!(
            existing_games.iter().all(|g| g.finished),
            "Not all games from previous rounds are finished"
        );

        // Get game speed
        let game_speed = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => GameSpeed::Blitz,
            TimeMode::Untimed => {
                return Err(DbError::InvalidInput {
                    info: String::from("Cannot start untimed tournament"),
                    error: String::from("Tournament must have a time mode"),
                });
            }
        };

        // Initialize player states with history
        let mut player_states: Vec<PlayerState> = Vec::new();
        for (i, player) in players.iter().enumerate() {
            let rating = Rating::for_uuid(&player.id, &game_speed, conn).await?;
            let mut state = PlayerState::new(player.clone(), rating.rating, (i + 1) as i32);

            // Calculate score and build history
            for game in &existing_games {
                if game.white_id == player.id {
                    state.opponents.insert(game.black_id);
                    state.colors.push(Color::White);
                    match TournamentGameResult::from_str(&game.tournament_game_result) {
                        Ok(TournamentGameResult::Winner(Color::White)) => state.score += WIN_SCORE,
                        Ok(TournamentGameResult::Draw) => state.score += DRAW_SCORE,
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
                    state.opponents.insert(game.white_id);
                    state.colors.push(Color::Black);
                    match TournamentGameResult::from_str(&game.tournament_game_result) {
                        Ok(TournamentGameResult::Winner(Color::Black)) => state.score += WIN_SCORE,
                        Ok(TournamentGameResult::Draw) => state.score += DRAW_SCORE,
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
            state.update_color_preference();
            player_states.push(state);
        }

        // Verify player histories
        for state in &player_states {
            // Verify color count difference is not more than 2
            let whites = state.colors.iter().filter(|&c| *c == Color::White).count();
            let blacks = state.colors.iter().filter(|&c| *c == Color::Black).count();
            debug_assert!(
                (whites as i32 - blacks as i32).abs() <= 2,
                "Player {} has invalid color balance: {} whites, {} blacks",
                state.player.username,
                whites,
                blacks
            );

            // Verify no duplicate opponents
            debug_assert_eq!(
                state.opponents.len(),
                state.colors.len(),
                "Player {} has incorrect number of opponents",
                state.player.username
            );
        }

        // Sort players by score and then by initial pairing number
        let mut player_states = player_states;
        player_states.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap()
                .then_with(|| a.pairing_number.cmp(&b.pairing_number))
        });

        // Verify sorting
        debug_assert!(
            player_states.windows(2).all(|w| {
                w[0].score > w[1].score
                    || (w[0].score == w[1].score && w[0].pairing_number <= w[1].pairing_number)
            }),
            "Players not properly sorted by score and pairing number"
        );

        // Group players by score
        let mut score_groups: Vec<Vec<PlayerState>> = Vec::new();
        let mut current_group: Vec<PlayerState> = Vec::new();
        let mut current_score = player_states[0].score;

        for player_state in player_states.iter() {
            if (player_state.score - current_score).abs() < SCORE_COMPARISON_EPSILON {
                current_group.push(player_state.clone());
            } else {
                if !current_group.is_empty() {
                    score_groups.push(current_group);
                }
                current_group = vec![player_state.clone()];
                current_score = player_state.score;
            }
        }
        if !current_group.is_empty() {
            score_groups.push(current_group);
        }

        // Handle odd number of players
        if players.len() % 2 != 0 {
            if let Some(bye_player_id) = self
                .find_bye_player_in_score_groups(&score_groups, conn)
                .await?
            {
                // Remove the bye player from their score group
                for group in &mut score_groups {
                    if let Some(pos) = group.iter().position(|p| p.player.id == bye_player_id) {
                        let bye_player = group.remove(pos);
                        println!(
                            "Giving bye to {} (score: {:.1})",
                            bye_player.player.username, bye_player.score
                        );

                        // Update tournament bye list
                        let mut new_byes = self.bye.clone();
                        new_byes.push(Some(bye_player.player.id));
                        diesel::update(tournaments::table.find(self.id))
                            .set(bye.eq(new_byes))
                            .execute(conn)
                            .await?;
                        break;
                    }
                }
            }
        } else {
            let mut new_byes = self.bye.clone();
            new_byes.push(None);
            diesel::update(tournaments::table.find(self.id))
                .set(bye.eq(new_byes))
                .execute(conn)
                .await?;
        }

        // Process each score group
        let mut score_groups_clone = score_groups.clone(); // Clone for later verification
        
        // Track players who need to float down
        let mut floating_players: Vec<PlayerState> = Vec::new();
        
        // Process score groups in order from highest to lowest score
        for group_index in 0..score_groups.len() {
            // Get the current group
            if group_index >= score_groups.len() {
                continue;
            }
            
            let mut group = score_groups[group_index].clone();
            
            // Skip empty groups
            if group.is_empty() {
                continue;
            }
            
            // Add any floating players from higher score groups
            group.append(&mut floating_players);
            floating_players = Vec::new(); // Clear the list after adding
            
            println!(
                "\nProcessing score group {} with {} players at {:.1} points:",
                group_index + 1,
                group.len(),
                if !group.is_empty() { group[0].score } else { 0.0 }
            );

            // Sort players within group by pairing number
            group.sort_by_key(|p| p.pairing_number);

            // Keep processing until we have fewer than 2 players
            while group.len() >= 2 {
                // Try to find a compatible opponent
                let mut paired = false;
                
                // Store data from the first player to avoid borrowing issues later
                let current_player_id = group[0].player.id;
                let current_player_username = group[0].player.username.clone();
                let current_player_score = group[0].score;
                
                let mut opponent_idx = 0;
                let mut opponent_id = Uuid::nil();
                let mut opponent_username = String::new();
                let mut opponent_score = 0.0;
                let mut white_id = Uuid::nil();
                let mut black_id = Uuid::nil();
                let mut white_username = String::new();
                let mut black_username = String::new();
                let mut white_score = 0.0;
                let mut black_score = 0.0;
                
                // First find a compatible opponent
                for i in 1..group.len() {
                    if self.are_compatible_opponents(&group[0], &group[i]) {
                        opponent_idx = i;
                        opponent_id = group[i].player.id;
                        opponent_username = group[i].player.username.clone();
                        opponent_score = group[i].score;
                        
                        // Determine colors
                        let (color1, _) = self.determine_colors(&group[0], &group[i]);
                        
                        if color1 == Color::White {
                            white_id = current_player_id;
                            black_id = opponent_id;
                            white_username = current_player_username.clone();
                            black_username = opponent_username.clone();
                            white_score = current_player_score;
                            black_score = opponent_score;
                        } else {
                            white_id = opponent_id;
                            black_id = current_player_id;
                            white_username = opponent_username.clone();
                            black_username = current_player_username.clone();
                            white_score = opponent_score;
                            black_score = current_player_score;
                        }
                        
                        paired = true;
                        break;
                    }
                }
                
                if paired {
                    println!(
                        "Pairing {} (White, score: {:.1}) vs {} (Black, score: {:.1})",
                        white_username, white_score, black_username, black_score
                    );
                    
                    // Create the game
                    let new_game = NewGame::new_from_tournament(white_id, black_id, self);
                    let game = Game::create(new_game, conn).await?;
                    games.push(game);
                    
                    // Remove paired players from the group
                    group.remove(opponent_idx);
                    group.remove(0);
                    
                    // Also update the original score_groups
                    if group_index < score_groups.len() {
                        score_groups[group_index].retain(|p| p.player.id != current_player_id && p.player.id != opponent_id);
                    }
                    
                    continue;
                }
                
                // If we have at least 2 players left, force a pairing
                if group.len() >= 2 {
                    // Store data to avoid borrowing conflicts
                    let p1_id = group[0].player.id;
                    let p2_id = group[1].player.id;
                    let p1_username = group[0].player.username.clone();
                    let p2_username = group[1].player.username.clone();
                    let p1_score = group[0].score;
                    let p2_score = group[1].score;
                    
                    // Determine colors
                    let (color1, _) = self.determine_colors(&group[0], &group[1]);
                    
                    if color1 == Color::White {
                        white_id = p1_id;
                        black_id = p2_id;
                        white_username = p1_username;
                        black_username = p2_username;
                        white_score = p1_score;
                        black_score = p2_score;
                    } else {
                        white_id = p2_id;
                        black_id = p1_id;
                        white_username = p2_username;
                        black_username = p1_username;
                        white_score = p2_score;
                        black_score = p1_score;
                    }
                    
                    println!(
                        "Forcing pairing of {} (White, score: {:.1}) vs {} (Black, score: {:.1})",
                        white_username, white_score, black_username, black_score
                    );
                    
                    // Create the game
                    let new_game = NewGame::new_from_tournament(white_id, black_id, self);
                    let game = Game::create(new_game, conn).await?;
                    games.push(game);
                    
                    // Remove paired players from the group
                    group.remove(1);
                    group.remove(0);
                    
                    // Also update the original score_groups
                    if group_index < score_groups.len() {
                        score_groups[group_index].retain(|p| p.player.id != p1_id && p.player.id != p2_id);
                    }
                    
                    continue;
                }
                
                // If we reach here, we have a lone player that needs to float down
                let floater = group.remove(0);
                
                // Also update the original score_groups
                if group_index < score_groups.len() {
                    score_groups[group_index].retain(|p| p.player.id != floater.player.id);
                }
                
                println!(
                    "No compatible opponent found for {} in current score group, will float to next group",
                    floater.player.username
                );
                
                // Add the player to the floating list
                floating_players.push(floater);
            }
            
            // Add any remaining players to the floating list
            floating_players.append(&mut group);
        }
        
        // Handle any remaining floating players
        while floating_players.len() >= 2 {
            let p1 = floating_players.remove(0);
            let p2 = floating_players.remove(0);
            
            // Determine colors
            let (color1, _) = self.determine_colors(&p1, &p2);
            
            let (white_id, black_id, white_name, black_name, white_score, black_score) = 
                if color1 == Color::White {
                    (p1.player.id, p2.player.id, 
                     p1.player.username.clone(), p2.player.username.clone(),
                     p1.score, p2.score)
                } else {
                    (p2.player.id, p1.player.id,
                     p2.player.username.clone(), p1.player.username.clone(), 
                     p2.score, p1.score)
                };
            
            println!(
                "Final pairing of floating players: {} (White, score: {:.1}) vs {} (Black, score: {:.1})",
                white_name, white_score, black_name, black_score
            );
            
            // Create the game
            let new_game = NewGame::new_from_tournament(white_id, black_id, self);
            let game = Game::create(new_game, conn).await?;
            games.push(game);
        }
        
        // Warn if there's an odd player out (which shouldn't happen if bye was assigned correctly)
        if !floating_players.is_empty() {
            println!(
                "Warning: {} players still unpaired after pairing process. First unpaired: {}",
                floating_players.len(),
                floating_players[0].player.username
            );
        }

        // Additional verifications for score groups
        for group in &score_groups_clone {
            // Skip empty groups
            if group.is_empty() {
                continue;
            }
            
            // Verify score group consistency
            let group_score = group[0].score;
            debug_assert!(
                group
                    .iter()
                    .all(|p| (p.score - group_score).abs() < f64::EPSILON),
                "Players in score group have different scores"
            );

            // Verify sorting within group
            debug_assert!(
                group
                    .windows(2)
                    .all(|w| w[0].pairing_number <= w[1].pairing_number),
                "Players in score group not properly sorted by pairing number"
            );
        }

        // Verify score groups are properly ordered
        debug_assert!(
            score_groups_clone
                .windows(2)
                .all(|w| w[0].is_empty() || w[1].is_empty() || w[0][0].score > w[1][0].score),
            "Score groups not properly ordered"
        );

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

        debug_assert_eq!(
            games.len(),
            players.len() / 2,
            "Incorrect number of games created. Expected {}, got {}",
            players.len() / 2,
            games.len()
        );

        Ok((tournament, games))
    }
}
