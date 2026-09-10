use crate::{
    db_error::DbError,
    helpers::GameQueryBuilder,
    models::{Challenge, GameFinishContext, GameHash, GameUser, Rating},
    schema::{
        challenges::{self, nanoid as nanoid_field},
        games::{self, dsl::*},
        games_users,
    },
    tournaments::game_time_parts,
    DbConn,
};
use ::nanoid::nanoid;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use diesel::{prelude::*, ExpressionMethods, Insertable};
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameControl, GameResult, GameStatus, GameType, State};
use serde::{Deserialize, Serialize};
use shared_types::{
    tournament::Slot,
    BatchToken,
    ChallengeId,
    Clock,
    Conclusion,
    CorrespondenceClock,
    GameId,
    GameSortKey,
    GameSpeed,
    GameStart,
    GamesQueryOptions,
    RealtimeClock,
    SortValue,
    TimeMode,
    TournamentGameResult,
};
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

pub static NANOS_IN_SECOND: i64 = 1_000_000_000_i64;

/// Named None for clearing timeout_at at terminal transitions; avoids
/// repeating the type ascription diesel's set-tuple inference needs.
pub(crate) const CLEAR_TIMEOUT_AT: Option<DateTime<Utc>> = None;

/// Single source of truth for timeout_at, so every site that mutates
/// clock/turn/status derives it consistently.
fn compute_timeout_at(
    interaction_at: Option<DateTime<Utc>>,
    white_left_nanos: Option<i64>,
    black_left_nanos: Option<i64>,
    new_turn: i32,
    mode_str: &str,
    status_str: &str,
) -> Option<DateTime<Utc>> {
    if status_str == GameStatus::NotStarted.to_string() {
        return None;
    }
    if matches!(TimeMode::from_str(mode_str), Ok(TimeMode::Untimed)) {
        return None;
    }
    let last = interaction_at?;
    let running_nanos = if new_turn % 2 == 0 {
        white_left_nanos?
    } else {
        black_left_nanos?
    };
    Some(last + ChronoDuration::nanoseconds(running_nanos))
}

/// `Repetition` only when the repetition is what ended it: replay records repetitions without
/// adjudicating, so a grandfathered game can carry an earlier one and still end any other way.
fn conclusion_for(status: &GameStatus, repeating_moves: &[usize], plies: usize) -> Conclusion {
    let ended_by_repetition = hive_lib::threefold_on_final_ply(repeating_moves, plies);
    match status {
        GameStatus::Finished(GameResult::Draw) if ended_by_repetition => Conclusion::Repetition,
        GameStatus::Finished(GameResult::Draw | GameResult::Winner(_)) => Conclusion::Board,
        _ => Conclusion::Unknown,
    }
}

#[derive(Debug)]
struct TimeInfo {
    white_time_left: Option<i64>,
    black_time_left: Option<i64>,
    new_game_status: GameStatus,
}

impl TimeInfo {
    pub fn new(status: GameStatus) -> Self {
        Self {
            white_time_left: None,
            black_time_left: None,
            new_game_status: status,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct GameRating {
    pub rating: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = games)]
pub struct NewGame {
    pub nanoid: String,
    pub current_player_id: Uuid,
    pub black_id: Uuid, // uid of user
    pub finished: bool,
    pub game_status: String,
    pub game_type: String,
    pub history: String,
    pub game_control_history: String,
    pub rated: bool,
    pub tournament_queen_rule: bool,
    pub turn: i32,
    pub white_id: Uuid, // uid of user
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_rating_change: Option<f64>,
    pub black_rating_change: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_mode: String,           // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Seconds
    pub time_increment: Option<i32>, // Seconds
    pub last_interaction: Option<DateTime<Utc>>, // When was the last move made
    pub black_time_left: Option<i64>, // A duration of nanos represented as an int
    pub white_time_left: Option<i64>, // A duration of nanos represented as an int
    pub speed: String,
    pub hashes: Vec<Option<i64>>,
    pub conclusion: String,
    pub tournament_game_result: String,
    pub game_start: String,
    pub move_times: Vec<Option<i64>>,
    pub timeout_at: Option<DateTime<Utc>>,
    pub tournament_id: Option<Uuid>,
    pub white_berserked: bool,
    pub black_berserked: bool,
    pub arena_move_due_at: Option<DateTime<Utc>>,
    pub tournament_slot_id: Option<Uuid>,
    pub arena_ordinal: Option<i64>,
}

impl NewGame {
    pub(crate) fn for_arena(
        owning_tournament_id: Uuid,
        ordinal: i64,
        white: Uuid,
        black: Uuid,
        clock: RealtimeClock,
        paired_at: DateTime<Utc>,
    ) -> Result<Self, DbError> {
        let (_, stored_base, stored_increment) =
            game_time_parts(Clock::Realtime(clock)).map_err(|error| {
                DbError::InvalidPersistedTournament {
                    reason: format!(
                        "Arena tournament {owning_tournament_id} has an invalid game clock: {error}"
                    ),
                }
            })?;
        let time_left = i64::from(clock.base_seconds.get()) * NANOS_IN_SECOND;
        Ok(Self {
            nanoid: nanoid!(12),
            current_player_id: white,
            black_id: black,
            finished: false,
            game_status: GameStatus::InProgress.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: true,
            turn: 0,
            white_id: white,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: paired_at,
            updated_at: paired_at,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: stored_base,
            time_increment: stored_increment,
            last_interaction: None,
            black_time_left: Some(time_left),
            white_time_left: Some(time_left),
            speed: GameSpeed::from(Clock::Realtime(clock)).to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Arena.to_string(),
            move_times: Vec::new(),
            timeout_at: None,
            tournament_id: Some(owning_tournament_id),
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: Some(paired_at + ChronoDuration::seconds(30)),
            tournament_slot_id: None,
            arena_ordinal: Some(ordinal),
        })
    }

    pub(crate) fn for_tournament_slot(
        owning_tournament_id: Uuid,
        slot: &Slot,
        now: DateTime<Utc>,
    ) -> Result<Self, DbError> {
        let (stored_time_mode, stored_time_base, stored_time_increment) =
            game_time_parts(slot.clock).map_err(|error| DbError::InvalidPersistedTournament {
                reason: format!(
                    "tournament Slot {} has an invalid game clock: {error}",
                    slot.id
                ),
            })?;
        let time_left_seconds = match slot.clock {
            Clock::Realtime(clock) => clock.base_seconds.get(),
            Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => {
                seconds_each.get()
            }
            Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => {
                seconds_per_move.get()
            }
        };
        let time_left = Some(i64::from(time_left_seconds) * NANOS_IN_SECOND);
        let (start_kind, initial_status, initial_interaction) = match slot.clock {
            Clock::Realtime(_) => (GameStart::Ready, GameStatus::NotStarted.to_string(), None),
            Clock::Correspondence(_) => (
                GameStart::Immediate,
                GameStatus::InProgress.to_string(),
                Some(now),
            ),
        };
        let stored_time_mode = stored_time_mode.to_string();
        let initial_timeout = compute_timeout_at(
            initial_interaction,
            time_left,
            time_left,
            0,
            &stored_time_mode,
            &initial_status,
        );

        Ok(Self {
            nanoid: nanoid!(12),
            current_player_id: slot.white,
            black_id: slot.black,
            finished: false,
            game_status: initial_status,
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: true,
            turn: 0,
            white_id: slot.white,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: stored_time_mode,
            time_base: stored_time_base,
            time_increment: stored_time_increment,
            last_interaction: initial_interaction,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::from(slot.clock).to_string(),
            hashes: vec![],
            conclusion: Conclusion::Unknown.to_string(),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: start_kind.to_string(),
            move_times: vec![],
            timeout_at: initial_timeout,
            tournament_id: Some(owning_tournament_id),
            white_berserked: false,
            black_berserked: false,
            // Non-Arena tournament games never carry an Arena opening deadline.
            arena_move_due_at: None,
            tournament_slot_id: Some(slot.id),
            arena_ordinal: None,
        })
    }

    pub fn new(white: Uuid, black: Uuid, challenge: &Challenge) -> Result<Self, DbError> {
        if white == black {
            return Err(DbError::InvalidInput {
                info: "You can't play here with yourself.".to_string(),
                error: String::new(),
            });
        }

        let clock = challenge.time_control()?;
        let game_speed = clock.map_or(GameSpeed::Untimed, GameSpeed::from);
        let time_left = match clock {
            None => None,
            Some(Clock::Realtime(clock)) => {
                Some(i64::from(clock.base_seconds.get()) * NANOS_IN_SECOND)
            }
            Some(Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each })) => {
                Some(i64::from(seconds_each.get()) * NANOS_IN_SECOND)
            }
            Some(Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move })) => {
                Some(i64::from(seconds_per_move.get()) * NANOS_IN_SECOND)
            }
        };

        Ok(Self {
            nanoid: challenge.nanoid.to_owned(),
            current_player_id: white,
            black_id: black,
            finished: false,
            game_status: "NotStarted".to_owned(),
            game_type: challenge.game_type.to_owned(),
            history: String::new(),
            game_control_history: String::new(),
            rated: challenge.rated,
            tournament_queen_rule: challenge.tournament_queen_rule,
            turn: 0,
            white_id: white,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            time_mode: challenge.time_mode.to_owned(),
            time_base: challenge.time_base,
            time_increment: challenge.time_increment,
            last_interaction: None,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: game_speed.to_string(),
            hashes: vec![],
            conclusion: Conclusion::Unknown.to_string(),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: vec![],
            timeout_at: None,
            tournament_id: None,
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: None,
            tournament_slot_id: None,
            arena_ordinal: None,
        })
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = games)]
pub struct Game {
    pub id: Uuid,
    pub nanoid: String,
    pub current_player_id: Uuid,
    pub black_id: Uuid, // uid of user
    pub finished: bool,
    pub game_status: String,
    pub game_type: String,
    pub history: String, //"piece pos;piece pos;piece pos;"
    pub game_control_history: String,
    pub rated: bool,
    pub tournament_queen_rule: bool,
    pub turn: i32,
    pub white_id: Uuid, // uid of user
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_rating_change: Option<f64>,
    pub black_rating_change: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_mode: String,           // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Seconds
    pub time_increment: Option<i32>, // Seconds
    pub last_interaction: Option<DateTime<Utc>>, // When was the last move made
    pub black_time_left: Option<i64>,
    pub white_time_left: Option<i64>,
    pub speed: String,
    hashes: Vec<Option<i64>>,
    pub conclusion: String,
    pub tournament_id: Option<Uuid>,
    pub tournament_game_result: String,
    pub game_start: String,
    pub move_times: Vec<Option<i64>>,
    pub timeout_at: Option<DateTime<Utc>>,
    pub white_berserked: bool,
    pub black_berserked: bool,
    /// When the game finished, as opposed to when the row was last written.
    /// The arena replays its timeline from stored instants, so it needs one
    /// that later writes to the row cannot move.
    pub finished_at: Option<DateTime<Utc>>,
    /// The active opening deadline for an Arena game's first two moves.
    pub arena_move_due_at: Option<DateTime<Utc>>,
    pub tournament_slot_id: Option<Uuid>,
    pub arena_ordinal: Option<i64>,
}

pub(crate) enum DeadlineSettlement {
    Active(Game),
    Terminal {
        game: Game,
        terminal_at: DateTime<Utc>,
    },
}

impl Game {
    pub fn hashes(&self) -> Vec<u64> {
        self.hashes
            .iter()
            .filter_map(|o| o.map(|i| i as u64))
            .collect()
    }

    pub async fn create(new_game: NewGame, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        let game: Game = new_game.insert_into(games::table).get_result(conn).await?;
        let game_user_white = GameUser::new(game.id, game.white_id);
        game_user_white
            .insert_into(games_users::table)
            .execute(conn)
            .await?;
        let game_user_black = GameUser::new(game.id, game.black_id);
        game_user_black
            .insert_into(games_users::table)
            .execute(conn)
            .await?;
        Ok(game)
    }

    pub async fn create_and_delete_challenges(
        new_game: NewGame,
        conn: &mut DbConn<'_>,
    ) -> Result<(Game, Vec<ChallengeId>), DbError> {
        let game = Game::create(new_game, conn).await?;
        let challenge: Challenge = challenges::table
            .filter(nanoid_field.eq(game.nanoid.clone()))
            .first(conn)
            .await?;
        let mut deleted = vec![];
        if let Ok(TimeMode::RealTime) = TimeMode::from_str(&challenge.time_mode) {
            let challenges: Vec<Challenge> = challenges::table
                .filter(
                    challenges::time_mode
                        .eq(TimeMode::RealTime.to_string())
                        .and(challenges::challenger_id.eq_any(&[game.white_id, game.black_id])),
                )
                .get_results(conn)
                .await?;
            for challenge in challenges {
                deleted.push(ChallengeId(challenge.nanoid));
                diesel::delete(challenges::table.find(challenge.id))
                    .execute(conn)
                    .await?;
            }
        } else {
            deleted.push(ChallengeId(challenge.nanoid));
            diesel::delete(challenges::table.find(challenge.id))
                .execute(conn)
                .await?;
        };
        Ok((game, deleted))
    }

    pub fn get_heartbeat(&self) -> Result<(GameId, Duration, Duration), DbError> {
        let (white, black) = self.get_time_left()?;
        Ok((GameId(self.nanoid.clone()), white, black))
    }

    pub fn get_time_left(&self) -> Result<(Duration, Duration), DbError> {
        let white = self.time_left_duration(Color::White)?;
        let black = self.time_left_duration(Color::Black)?;
        if self.game_status == GameStatus::NotStarted.to_string() {
            return Ok((white, black));
        }
        if let Some(last) = self.last_interaction {
            if let Ok(time_passed) = Utc::now().signed_duration_since(last).to_std() {
                if self.turn % 2 == 0 {
                    if white < time_passed {
                        return Ok((Duration::from_secs(0), black));
                    }
                    return Ok((white - time_passed, black));
                } else {
                    if black < time_passed {
                        return Ok((white, Duration::from_secs(0)));
                    }
                    return Ok((white, black - time_passed));
                };
            }
        }
        Ok((white, black))
    }

    pub(crate) async fn settle_deadline(
        &self,
        checked_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<DeadlineSettlement, DbError> {
        if self.finished || TimeMode::from_str(&self.time_mode)? == TimeMode::Untimed {
            return Ok(DeadlineSettlement::Active(self.clone()));
        }
        if let Some(deadline) = self
            .arena_move_due_at
            .filter(|deadline| checked_at >= *deadline)
        {
            let absent = if self.turn == 0 {
                Color::White
            } else {
                Color::Black
            };
            let game = self
                .finish_arena_no_start(absent, checked_at, deadline, conn)
                .await?;
            return Ok(DeadlineSettlement::Terminal {
                game,
                terminal_at: deadline,
            });
        }
        if let Some(deadline) = self.timeout_at.filter(|deadline| checked_at >= *deadline) {
            let timed_out_color = if self.turn % 2 == 0 {
                Color::White
            } else {
                Color::Black
            };
            let game = self
                .finish_timeout(timed_out_color, checked_at, deadline, conn)
                .await?;
            return Ok(DeadlineSettlement::Terminal {
                game,
                terminal_at: deadline,
            });
        }
        Ok(DeadlineSettlement::Active(self.clone()))
    }

    async fn finish_timeout(
        &self,
        timed_out_color: Color,
        checked_at: DateTime<Utc>,
        terminal_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let result = GameResult::Winner(timed_out_color.opposite_color());
        let (new_white_time_left, new_black_time_left) = match timed_out_color {
            Color::White => (Some(0_i64), self.black_time_left),
            Color::Black => (self.white_time_left, Some(0_i64)),
        };
        let tgr = TournamentGameResult::new(&result);
        let new_game_status = GameStatus::Finished(result.clone());
        let (
            white_rating_before,
            black_rating_before,
            new_white_rating_change,
            new_black_rating_change,
        ) = Rating::update(
            self.rated,
            self.speed.clone(),
            self.white_id,
            self.black_id,
            result,
            checked_at,
            conn,
        )
        .await?;
        let game: Game = diesel::update(games::table.find(self.id))
            .set((
                games::finished.eq(true),
                games::tournament_game_result.eq(tgr.to_string()),
                games::game_status.eq(new_game_status.to_string()),
                games::white_rating.eq(white_rating_before),
                games::black_rating.eq(black_rating_before),
                games::white_rating_change.eq(new_white_rating_change),
                games::black_rating_change.eq(new_black_rating_change),
                games::updated_at.eq(checked_at),
                games::white_time_left.eq(new_white_time_left),
                games::black_time_left.eq(new_black_time_left),
                games::conclusion.eq(Conclusion::Timeout.to_string()),
                games::timeout_at.eq(CLEAR_TIMEOUT_AT),
                games::arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                games::finished_at.eq(Some(terminal_at)),
            ))
            .get_result(conn)
            .await?;
        // `games.hashes` needs no rewrite: this finish plays no move, so the array the last one
        // stored is still current. A rehash is the one thing that empties it mid-game, and
        // `hash_backfill` refills it on the next boot.
        let ctx = GameFinishContext::from_finished_game(&game);
        Self::insert_persisted_game_hashes(&game, &ctx, conn).await?;
        Ok(game)
    }

    pub(crate) async fn finish_game_control(
        &self,
        game_control: GameControl,
        result: GameResult,
        final_conclusion: Conclusion,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        let (new_white_time_left, new_black_time_left) = match TimeMode::from_str(&self.time_mode)?
        {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left_at(effective_at)?,
        };
        let tgr = TournamentGameResult::new(&result);
        let new_game_status = GameStatus::Finished(result.clone());
        let (
            white_rating_before,
            black_rating_before,
            new_white_rating_change,
            new_black_rating_change,
        ) = Rating::update(
            self.rated,
            self.speed.clone(),
            self.white_id,
            self.black_id,
            result,
            effective_at,
            conn,
        )
        .await?;
        let game: Game = diesel::update(games::table.find(self.id))
            .set((
                games::finished.eq(true),
                games::tournament_game_result.eq(tgr.to_string()),
                games::game_status.eq(new_game_status.to_string()),
                games::game_control_history
                    .eq(games::game_control_history.concat(game_control_string)),
                games::white_rating.eq(white_rating_before),
                games::black_rating.eq(black_rating_before),
                games::white_rating_change.eq(new_white_rating_change),
                games::black_rating_change.eq(new_black_rating_change),
                games::updated_at.eq(effective_at),
                games::white_time_left.eq(new_white_time_left),
                games::black_time_left.eq(new_black_time_left),
                games::conclusion.eq(final_conclusion.to_string()),
                games::last_interaction.eq(Some(effective_at)),
                games::timeout_at.eq(CLEAR_TIMEOUT_AT),
                games::arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                games::finished_at.eq(Some(effective_at)),
            ))
            .get_result(conn)
            .await?;
        let ctx = GameFinishContext::from_finished_game(&game);
        Self::insert_persisted_game_hashes(&game, &ctx, conn).await?;
        Ok(game)
    }

    async fn insert_persisted_game_hashes(
        game: &Game,
        context: &GameFinishContext,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let state = State::new_from_str(&game.history, &game.game_type).map_err(|error| {
            DbError::InternalError {
                reason: format!("game history does not replay at finalization: {error}"),
            }
        })?;
        GameHash::insert_for_game(game.id, &state.hashes, &state.history.moves, context, conn).await
    }

    pub fn time_left_duration(&self, color: Color) -> Result<Duration, DbError> {
        let (time_left, missing_field) = match color {
            Color::White => (self.white_time_left, "white_time"),
            Color::Black => (self.black_time_left, "black_time"),
        };

        time_left
            .map(|time| Duration::from_nanos(time as u64))
            .ok_or_else(|| DbError::TimeNotFound {
                reason: format!("Could not find {missing_field}"),
            })
    }

    pub fn berserked(&self, color: Color) -> bool {
        match color {
            Color::White => self.white_berserked,
            Color::Black => self.black_berserked,
        }
    }

    pub(crate) fn is_arena_no_start(&self) -> bool {
        self.finished && self.turn < 2 && self.conclusion == Conclusion::Timeout.to_string()
    }

    pub(crate) async fn declare_berserk(
        &self,
        color: Color,
        clock: RealtimeClock,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let eligible_turn = match color {
            Color::White => self.turn == 0,
            Color::Black => matches!(self.turn, 0 | 1),
        };
        if self.game_start != GameStart::Arena.to_string()
            || self.finished
            || !eligible_turn
            || self.arena_move_due_at.is_none()
            || self.berserked(color)
        {
            return Err(DbError::InvalidAction {
                info: String::from("Berserk must be declared before that Arena player's move"),
            });
        }
        let penalty = Self::berserk_penalty_nanos(clock);
        let starting_time = i64::from(clock.base_seconds.get()) * NANOS_IN_SECOND;
        let next_time = Some(starting_time - penalty);
        Ok(match color {
            Color::White => {
                diesel::update(games::table.find(self.id))
                    .set((
                        games::white_berserked.eq(true),
                        games::white_time_left.eq(next_time),
                        games::updated_at.eq(effective_at),
                    ))
                    .get_result(conn)
                    .await?
            }
            Color::Black => {
                diesel::update(games::table.find(self.id))
                    .set((
                        games::black_berserked.eq(true),
                        games::black_time_left.eq(next_time),
                        games::updated_at.eq(effective_at),
                    ))
                    .get_result(conn)
                    .await?
            }
        })
    }

    async fn finish_arena_no_start(
        &self,
        absent: Color,
        checked_at: DateTime<Utc>,
        terminal_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let result = GameResult::Winner(absent.opposite_color());
        let (
            white_rating_before,
            black_rating_before,
            next_white_rating_change,
            next_black_rating_change,
        ) = Rating::update(
            true,
            self.speed.clone(),
            self.white_id,
            self.black_id,
            result.clone(),
            checked_at,
            conn,
        )
        .await?;
        let (white_left, black_left) = match absent {
            Color::White => (Some(0), self.black_time_left),
            Color::Black => (self.white_time_left, Some(0)),
        };
        let updated: Game = diesel::update(games::table.find(self.id))
            .set((
                games::finished.eq(true),
                games::game_status.eq(GameStatus::Finished(result.clone()).to_string()),
                games::tournament_game_result.eq(TournamentGameResult::new(&result).to_string()),
                games::conclusion.eq(Conclusion::Timeout.to_string()),
                games::white_rating.eq(Some(white_rating_before)),
                games::black_rating.eq(Some(black_rating_before)),
                games::white_rating_change.eq(next_white_rating_change),
                games::black_rating_change.eq(next_black_rating_change),
                games::white_time_left.eq(white_left),
                games::black_time_left.eq(black_left),
                games::last_interaction.eq(Some(terminal_at)),
                games::updated_at.eq(checked_at),
                games::timeout_at.eq(CLEAR_TIMEOUT_AT),
                games::arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                games::finished_at.eq(Some(terminal_at)),
            ))
            .get_result(conn)
            .await?;
        let context = GameFinishContext::from_finished_game(&updated);
        Self::insert_persisted_game_hashes(&updated, &context, conn).await?;
        Ok(updated)
    }

    pub(crate) async fn find_due_arena_opening_ids(
        as_of: DateTime<Utc>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        Ok(games::table
            .filter(games::finished.eq(false))
            .filter(games::game_start.eq(GameStart::Arena.to_string()))
            .filter(games::tournament_id.is_not_null())
            .filter(games::arena_move_due_at.le(as_of))
            .order((games::arena_move_due_at.asc(), games::id.asc()))
            .limit(limit)
            .select(games::id)
            .load(conn)
            .await?)
    }

    pub(crate) async fn find_due_arena_timeout_ids(
        as_of: DateTime<Utc>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        Ok(games::table
            .filter(games::finished.eq(false))
            .filter(games::game_start.eq(GameStart::Arena.to_string()))
            .filter(games::tournament_id.is_not_null())
            .filter(games::arena_move_due_at.is_null())
            .filter(games::timeout_at.le(as_of))
            .order((games::timeout_at.asc(), games::id.asc()))
            .limit(limit)
            .select(games::id)
            .load(conn)
            .await?)
    }

    /// Berserking trades clock for arena points, and lichess charges it two
    /// ways: the increment goes, and half the base goes with it. The penalty is
    /// waived when the increment dominates the base, since halving a 10+10 game
    /// would be no penalty at all but losing the increment would be brutal.
    /// (lila `ClockConfig::berserkPenalty` — same `40 x increment` threshold
    /// `GameSpeed::from_base_increment` already uses.)
    pub fn berserk_penalty_nanos(clock: RealtimeClock) -> i64 {
        let base = i64::from(clock.base_seconds.get());
        if base < 40 * i64::from(clock.increment_seconds) {
            return 0;
        }
        base * NANOS_IN_SECOND / 2
    }

    /// The increment actually credited to `color`: none, if they berserked.
    fn time_increment_duration(&self, color: Color, clock: RealtimeClock) -> Duration {
        if self.berserked(color) {
            return Duration::ZERO;
        }
        Duration::from_secs(u64::from(clock.increment_seconds))
    }

    fn calculate_time_left_at(
        &self,
        observed_at: DateTime<Utc>,
    ) -> Result<(Option<i64>, Option<i64>), DbError> {
        let mut time_left = self.time_left_duration(if self.turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        })?;
        let (mut black_time, mut white_time) = (self.black_time_left, self.white_time_left);
        if let Some(last) = self.last_interaction {
            let time_passed = observed_at
                .signed_duration_since(last)
                .to_std()
                .map_err(|_| DbError::SerializationConflict)?;
            if time_left > time_passed {
                // substract passed time and add time_increment
                time_left -= time_passed;
                if self.turn % 2 == 0 {
                    white_time = Some(time_left.as_nanos() as i64);
                } else {
                    black_time = Some(time_left.as_nanos() as i64);
                };
            } else if self.turn % 2 == 0 {
                white_time = Some(0);
            } else {
                black_time = Some(0);
            }
        }
        Ok((white_time, black_time))
    }

    fn calculate_time_left_add_increment_at(
        &self,
        shutout: bool,
        comp: f64,
        clock: RealtimeClock,
        observed_at: DateTime<Utc>,
    ) -> Result<(Option<i64>, Option<i64>), DbError> {
        let (mut white_time, mut black_time) = self.calculate_time_left_at(observed_at)?;
        if let (Some(w), Some(b)) = (white_time, black_time) {
            if w == 0 || b == 0 {
                return Ok((white_time, black_time));
            }
        }
        let comp = (comp * 1_000_000_000.0) as i64;
        // Each side carries its own increment, because a berserked player
        // forfeits theirs while the opponent keeps hers.
        let white_increment = self.time_increment_duration(Color::White, clock).as_nanos() as i64;
        let black_increment = self.time_increment_duration(Color::Black, clock).as_nanos() as i64;
        if self.turn % 2 == 0 {
            white_time = white_time.map(|time| time + white_increment + comp);
        } else {
            black_time = black_time.map(|time| time + black_increment + comp);
        };
        if shutout {
            if self.turn % 2 == 0 {
                black_time = black_time.map(|time| time + black_increment);
            } else {
                white_time = white_time.map(|time| time + white_increment);
            };
        };

        Ok((white_time, black_time))
    }

    fn get_time_info_at(
        &self,
        state: &State,
        comp: f64,
        observed_at: DateTime<Utc>,
    ) -> Result<TimeInfo, DbError> {
        let mode = TimeMode::from_str(&self.time_mode)?;
        let clock =
            Clock::from_time_parts(mode, self.time_base, self.time_increment).map_err(|error| {
                DbError::InternalError {
                    reason: format!("game has an invalid persisted time control: {error}"),
                }
            })?;
        match clock {
            None => Ok(TimeInfo::new(state.game_status.clone())),
            Some(Clock::Realtime(clock)) => {
                self.get_realtime_time_info_at(state, comp, clock, observed_at)
            }
            Some(Clock::Correspondence(clock)) => {
                self.get_correspondence_time_info_at(state, clock, observed_at)
            }
        }
    }

    fn get_realtime_time_info_at(
        &self,
        state: &State,
        comp: f64,
        clock: RealtimeClock,
        observed_at: DateTime<Utc>,
    ) -> Result<TimeInfo, DbError> {
        let mut time_info = TimeInfo::new(state.game_status.clone());
        if self.turn < 2
            && self.game_start == GameStart::Moves.to_string()
            && self.game_status == GameStatus::NotStarted.to_string()
        {
            if self.turn == 0 {
                time_info.new_game_status = GameStatus::NotStarted;
            };
            time_info.white_time_left = self.white_time_left;
            time_info.black_time_left = self.black_time_left;
        } else {
            (time_info.white_time_left, time_info.black_time_left) = self
                .calculate_time_left_add_increment_at(
                    state.history.last_move_is_pass(),
                    comp,
                    clock,
                    observed_at,
                )?;
        }
        Ok(time_info)
    }

    fn get_correspondence_time_info_at(
        &self,
        state: &State,
        clock: CorrespondenceClock,
        observed_at: DateTime<Utc>,
    ) -> Result<TimeInfo, DbError> {
        let mut time_info = TimeInfo::new(state.game_status.clone());
        if self.turn < 2
            && self.game_start == GameStart::Moves.to_string()
            && self.game_status == GameStatus::NotStarted.to_string()
        {
            if self.turn == 0 {
                time_info.new_game_status = GameStatus::NotStarted;
            };
            time_info.white_time_left = self.white_time_left;
            time_info.black_time_left = self.black_time_left;
        } else {
            (time_info.white_time_left, time_info.black_time_left) =
                self.calculate_time_left_at(observed_at)?;
            if let CorrespondenceClock::DaysPerMove { seconds_per_move } = clock {
                let move_time = i64::from(seconds_per_move.get()) * NANOS_IN_SECOND;
                if self.turn % 2 == 0 && time_info.white_time_left != Some(0) {
                    time_info.white_time_left = Some(move_time);
                    if state.history.last_move_is_pass() {
                        time_info.black_time_left = Some(move_time);
                    }
                } else if self.turn % 2 != 0 && time_info.black_time_left != Some(0) {
                    time_info.black_time_left = Some(move_time);
                    if state.history.last_move_is_pass() {
                        time_info.white_time_left = Some(move_time);
                    }
                }
            }
        }
        Ok(time_info)
    }

    fn get_move_times(&self, time_info: &TimeInfo, state: &State) -> Vec<Option<i64>> {
        let mut new_move_times = self.move_times.clone();
        if self.time_mode != TimeMode::Untimed.to_string() {
            if !state.history.last_move_is_pass() {
                // Not a shutout so we just add the players time
                if state.turn.is_multiple_of(2) {
                    new_move_times.push(time_info.black_time_left);
                } else {
                    new_move_times.push(time_info.white_time_left);
                }
            } else {
                // A shutout has happened, so state.turn was incremented twice so the "previous/not
                // shutout" player's time has to be added first. Note that we need to do it the
                // other way round than in if it's not a shutout
                if state.turn.is_multiple_of(2) {
                    new_move_times.push(time_info.white_time_left);
                } else {
                    new_move_times.push(time_info.black_time_left);
                }
                // Now the shutout player's time can be added
                if state.turn.is_multiple_of(2) {
                    new_move_times.push(time_info.black_time_left);
                } else {
                    new_move_times.push(time_info.white_time_left);
                }
            }
        }
        new_move_times
    }

    pub(crate) async fn update_gamestate(
        &self,
        state: &State,
        compensation: f64,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let arena_opening = self.game_start == GameStart::Arena.to_string() && self.turn < 2;
        let time_info = if arena_opening {
            let mut time_info = TimeInfo::new(state.game_status.clone());
            time_info.white_time_left = self.white_time_left;
            time_info.black_time_left = self.black_time_left;
            time_info
        } else {
            self.get_time_info_at(state, compensation, effective_at)?
        };
        let new_history = state
            .history
            .moves
            .iter()
            .map(|(piece, destination)| format!("{piece} {destination};"))
            .collect::<Vec<String>>()
            .join("");

        let game_control_string = match self.last_game_control()? {
            Some(GameControl::TakebackRequest(color)) => {
                let implicit = GameControl::TakebackReject(color.opposite_color());
                format!("{}. {implicit};", self.turn)
            }
            Some(GameControl::DrawOffer(color)) => {
                let implicit = GameControl::DrawReject(color.opposite_color());
                format!("{}. {implicit};", self.turn)
            }
            _ => String::new(),
        };

        let new_conclusion = conclusion_for(
            &time_info.new_game_status,
            &state.repeating_moves,
            state.hashes.len(),
        );

        let next_player = if state.turn.is_multiple_of(2) {
            self.white_id
        } else {
            self.black_id
        };

        let new_move_times = self.get_move_times(&time_info, state);
        let new_hashes: Vec<Option<i64>> = state.hashes.iter().map(|h| Some(*h as i64)).collect();

        if let GameStatus::Finished(game_result) = time_info.new_game_status.clone() {
            if let GameResult::Unknown = game_result {
                return Err(DbError::InternalError {
                    reason: String::from("engine finished a game without a result"),
                });
            };
            let new_turn = state.turn as i32;
            let new_white_time_left = time_info.white_time_left;
            let new_black_time_left = time_info.black_time_left;
            let tgr = TournamentGameResult::new(&game_result);
            let new_game_status = GameStatus::Finished(game_result.clone());
            let (
                white_rating_before,
                black_rating_before,
                new_white_rating_change,
                new_black_rating_change,
            ) = Rating::update(
                self.rated,
                self.speed.clone(),
                self.white_id,
                self.black_id,
                game_result,
                effective_at,
                conn,
            )
            .await?;
            let updated_game: Game = diesel::update(games::table.find(self.id))
                .set((
                    games::history.eq(new_history),
                    games::current_player_id.eq(next_player),
                    games::turn.eq(new_turn),
                    games::finished.eq(true),
                    games::tournament_game_result.eq(tgr.to_string()),
                    games::game_status.eq(new_game_status.to_string()),
                    games::game_control_history
                        .eq(games::game_control_history.concat(game_control_string)),
                    games::white_rating.eq(white_rating_before),
                    games::black_rating.eq(black_rating_before),
                    games::white_rating_change.eq(new_white_rating_change),
                    games::black_rating_change.eq(new_black_rating_change),
                    games::updated_at.eq(effective_at),
                    games::white_time_left.eq(new_white_time_left),
                    games::black_time_left.eq(new_black_time_left),
                    games::last_interaction.eq(Some(effective_at)),
                    games::move_times.eq(new_move_times),
                    games::hashes.eq(&new_hashes),
                    games::conclusion.eq(new_conclusion.to_string()),
                    games::timeout_at.eq(CLEAR_TIMEOUT_AT),
                    games::arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                    games::finished_at.eq(Some(effective_at)),
                ))
                .get_result(conn)
                .await?;
            let ctx = GameFinishContext::from_finished_game(&updated_game);
            GameHash::insert_for_game(
                updated_game.id,
                &state.hashes,
                &state.history.moves,
                &ctx,
                conn,
            )
            .await?;
            return Ok(updated_game);
        }

        let new_turn = state.turn as i32;
        let new_status_str = time_info.new_game_status.to_string();
        let (new_last_interaction, new_timeout_at, new_arena_move_due_at) =
            if arena_opening && self.turn == 0 {
                (None, None, Some(effective_at + ChronoDuration::seconds(30)))
            } else if arena_opening {
                (
                    Some(effective_at),
                    time_info
                        .white_time_left
                        .map(|time| effective_at + ChronoDuration::nanoseconds(time)),
                    None,
                )
            } else {
                (
                    Some(effective_at),
                    compute_timeout_at(
                        Some(effective_at),
                        time_info.white_time_left,
                        time_info.black_time_left,
                        new_turn,
                        &self.time_mode,
                        &new_status_str,
                    ),
                    self.arena_move_due_at,
                )
            };
        Ok(diesel::update(games::table.find(self.id))
            .set((
                history.eq(new_history),
                current_player_id.eq(next_player),
                turn.eq(new_turn),
                game_status.eq(new_status_str),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(effective_at),
                white_time_left.eq(time_info.white_time_left),
                black_time_left.eq(time_info.black_time_left),
                move_times.eq(new_move_times),
                last_interaction.eq(new_last_interaction),
                timeout_at.eq(new_timeout_at),
                arena_move_due_at.eq(new_arena_move_due_at),
                hashes.eq(new_hashes),
            ))
            .get_result(conn)
            .await?)
    }

    pub fn user_is_player(&self, user_id: Uuid) -> bool {
        self.white_id == user_id || self.black_id == user_id
    }

    pub fn user_color(&self, user_id: Uuid) -> Option<Color> {
        if user_id == self.black_id {
            return Some(Color::Black);
        }
        if user_id == self.white_id {
            return Some(Color::White);
        }
        None
    }

    pub fn last_game_control(&self) -> Result<Option<GameControl>, DbError> {
        let Some(last) = self.game_control_history.split_terminator(';').next_back() else {
            return Ok(None);
        };
        let encoded =
            last.split_whitespace()
                .next_back()
                .ok_or_else(|| DbError::InternalError {
                    reason: String::from("game control history contains an empty entry"),
                })?;
        GameControl::from_str(encoded)
            .map(Some)
            .map_err(|error| DbError::InternalError {
                reason: format!("game control history is invalid: {error}"),
            })
    }

    pub(crate) async fn write_game_control(
        &self,
        game_control: GameControl,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(effective_at),
            ))
            .get_result(conn)
            .await?)
    }

    fn get_takeback_time_correspondence(
        &self,
        popped: i32,
        clock: CorrespondenceClock,
    ) -> (Option<i64>, Option<i64>) {
        let CorrespondenceClock::DaysPerMove { seconds_per_move } = clock else {
            return self.get_takeback_time_realtime(popped, 0);
        };
        let move_time = i64::from(seconds_per_move.get()) * NANOS_IN_SECOND;
        let mut black_time = self.black_time_left;
        let mut white_time = self.white_time_left;

        if self.turn % 2 == 0 {
            black_time = Some(move_time);
        } else {
            white_time = Some(move_time);
        }

        if popped == 2 {
            if self.turn % 2 == 0 {
                white_time = Some(move_time);
            } else {
                black_time = Some(move_time);
            }
        }

        (white_time, black_time)
    }

    fn get_takeback_time_realtime(
        &self,
        popped: i32,
        increment_seconds: u32,
    ) -> (Option<i64>, Option<i64>) {
        let past_turn = self.turn - popped;
        let configured_increment = i64::from(increment_seconds) * NANOS_IN_SECOND;
        let white_increment =
            if self.game_start == GameStart::Arena.to_string() && self.white_berserked {
                0
            } else {
                configured_increment
            };
        let black_increment =
            if self.game_start == GameStart::Arena.to_string() && self.black_berserked {
                0
            } else {
                configured_increment
            };
        let mut times = self.move_times.clone();
        let mut black_time = self.black_time_left;
        let mut white_time = self.white_time_left;

        if self.turn % 2 == 0 {
            black_time = times.pop().flatten();
        } else {
            white_time = times.pop().flatten();
        }

        if popped == 2 {
            if self.turn % 2 == 0 {
                white_time = times.pop().flatten();
            } else {
                black_time = times.pop().flatten();
            }
        }

        if past_turn > 1 {
            if self.turn % 2 == 0 {
                black_time = black_time.map(|time| time.saturating_sub(black_increment));
            } else {
                white_time = white_time.map(|time| time.saturating_sub(white_increment));
            }
            if popped == 2 {
                if self.turn % 2 == 0 {
                    white_time = white_time.map(|time| time.saturating_sub(white_increment));
                } else {
                    black_time = black_time.map(|time| time.saturating_sub(black_increment));
                }
            }
        }
        (white_time, black_time)
    }

    fn get_takeback_time(&self, popped: i32) -> Result<(Option<i64>, Option<i64>), DbError> {
        let mode = TimeMode::from_str(&self.time_mode)?;
        let clock =
            Clock::from_time_parts(mode, self.time_base, self.time_increment).map_err(|error| {
                DbError::InternalError {
                    reason: format!("game has an invalid persisted time control: {error}"),
                }
            })?;
        match clock {
            None => Ok((None, None)),
            Some(Clock::Realtime(clock)) => {
                Ok(self.get_takeback_time_realtime(popped, clock.increment_seconds))
            }
            Some(Clock::Correspondence(clock)) => {
                Ok(self.get_takeback_time_correspondence(popped, clock))
            }
        }
    }

    pub(crate) async fn accept_takeback(
        &self,
        game_control: GameControl,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        let mut moves = self.history.split_terminator(';').collect::<Vec<_>>();
        let mut popped = 0_i32;
        let mut new_move_times = self.move_times.clone();

        if let Some(a_move) = moves.pop() {
            new_move_times.pop();
            popped += 1;
            if a_move.trim() == "pass" {
                moves.pop();
                new_move_times.pop();
                popped += 1;
            }
        }

        if popped == 0 {
            return Err(DbError::InvalidInput {
                info: String::from("Takeback failed, no moves to pop"),
                error: String::from("Popped = 0"),
            });
        }

        let (white_time, black_time) = self.get_takeback_time(popped)?;
        let mut new_history = moves.join(";");
        if !new_history.is_empty() {
            new_history.push(';');
        };

        let state = State::new_from_str(&new_history, &self.game_type).map_err(|error| {
            DbError::InternalError {
                reason: format!("game state cannot be rebuilt after takeback: {error}"),
            }
        })?;
        let new_turn = self.turn - popped;
        // Once a move-triggered game starts, takebacks do not re-arm its held-clock opening.
        let new_game_status = if self.game_start == GameStart::Moves.to_string() {
            self.game_status.clone()
        } else {
            state.game_status.to_string()
        };
        let next_player = if new_turn % 2 == 0 {
            self.white_id
        } else {
            self.black_id
        };
        let now = effective_at;
        let new_timeout_at = compute_timeout_at(
            Some(now),
            white_time,
            black_time,
            new_turn,
            &self.time_mode,
            &new_game_status,
        );

        Ok(diesel::update(games::table.find(self.id))
            .set((
                current_player_id.eq(next_player),
                history.eq(new_history),
                turn.eq(new_turn),
                game_status.eq(new_game_status),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(now),
                last_interaction.eq(now),
                move_times.eq(new_move_times),
                hashes.eq(state
                    .hashes
                    .iter()
                    .map(|h| Some(*h as i64))
                    .collect::<Vec<Option<i64>>>()),
                white_time_left.eq(white_time),
                black_time_left.eq(black_time),
                timeout_at.eq(new_timeout_at),
            ))
            .get_result(conn)
            .await?)
    }

    pub(crate) async fn adjudicate_unstarted(
        &self,
        new_result: &TournamentGameResult,
        adjudication_conclusion: Conclusion,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        if self.finished || self.turn != 0 || self.game_status != GameStatus::NotStarted.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot adjudicate a tournament game that has begun"),
            });
        }
        Ok(diesel::update(games::table.find(self.id))
            .set((
                finished.eq(true),
                rated.eq(false),
                conclusion.eq(adjudication_conclusion.to_string()),
                game_status.eq(GameStatus::Adjudicated.to_string()),
                tournament_game_result.eq(new_result.to_string()),
                updated_at.eq(effective_at),
                last_interaction.eq(Some(effective_at)),
                timeout_at.eq(CLEAR_TIMEOUT_AT),
                arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                finished_at.eq(Some(effective_at)),
            ))
            .get_result(conn)
            .await?)
    }

    pub(crate) async fn clear_adjudication(
        &self,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        if !self.finished
            || self.turn != 0
            || self.game_status != GameStatus::Adjudicated.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot clear this tournament game adjudication"),
            });
        }
        Ok(diesel::update(games::table.find(self.id))
            .set((
                finished.eq(false),
                rated.eq(true),
                conclusion.eq(Conclusion::Unknown.to_string()),
                game_status.eq(GameStatus::NotStarted.to_string()),
                tournament_game_result.eq(TournamentGameResult::Unknown.to_string()),
                updated_at.eq(effective_at),
                last_interaction.eq(Option::<DateTime<Utc>>::None),
                timeout_at.eq(CLEAR_TIMEOUT_AT),
                arena_move_due_at.eq(CLEAR_TIMEOUT_AT),
                finished_at.eq(Option::<DateTime<Utc>>::None),
            ))
            .get_result(conn)
            .await?)
    }

    pub(crate) async fn replace_adjudication(
        &self,
        result: &TournamentGameResult,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        if !self.finished
            || self.turn != 0
            || self.game_status != GameStatus::Adjudicated.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot replace this tournament game adjudication"),
            });
        }
        Ok(diesel::update(games::table.find(self.id))
            .set((
                conclusion.eq(Conclusion::Committee.to_string()),
                tournament_game_result.eq(result.to_string()),
                updated_at.eq(effective_at),
                last_interaction.eq(Some(effective_at)),
                finished_at.eq(Some(effective_at)),
            ))
            .get_result(conn)
            .await?)
    }

    pub(crate) async fn find_by_uuid_for_update(
        uuid: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        Ok(games::table
            .find(uuid)
            .for_update()
            .select(Self::as_select())
            .first(conn)
            .await?)
    }

    /// Locks the requested game rows in canonical UUID order. The caller must
    /// keep the surrounding transaction open for the locks to remain useful.
    pub(crate) async fn find_by_ids_for_update(
        game_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        if game_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(games::table
            .filter(id.eq_any(game_ids))
            .order(id.asc())
            .for_update()
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub async fn find_by_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        Ok(games::table
            .find(uuid)
            .select(Self::as_select())
            .first(conn)
            .await?)
    }

    pub async fn find_by_game_id(game_id: &GameId, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        Ok(games::table
            .filter(nanoid.eq(&game_id.0))
            .select(Self::as_select())
            .first(conn)
            .await?)
    }

    pub async fn find_by_game_ids(
        game_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(id.eq_any(game_ids))
            .order(id.asc())
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    /// Read-only batched lookup used by TV snapshots and websocket heartbeat.
    /// Heartbeat observes due deadlines through the explicit game-command
    /// path after this load; response construction never advances a tournament.
    pub async fn find_by_nanoids(
        game_ids: &[GameId],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let nanoids: Vec<&str> = game_ids.iter().map(|game_id| game_id.0.as_str()).collect();
        Ok(games::table
            .filter(nanoid.eq_any(nanoids))
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    /// In-flight games past their `timeout_at`. Uses the partial index, so
    /// near-free when none are due.
    pub async fn find_expired_by_timeout_at(
        as_of: DateTime<Utc>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(games::finished.eq(false))
            .filter(games::tournament_id.is_null())
            .filter(games::timeout_at.is_not_null())
            .filter(games::timeout_at.le(as_of))
            .order(games::timeout_at.asc())
            .limit(limit)
            .load(conn)
            .await?)
    }

    pub(crate) async fn delete(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::delete(games::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete_old_and_unstarted(conn: &mut DbConn<'_>) -> Result<(), DbError> {
        let cutoff = Utc::now() - Duration::from_secs(60 * 60 * 12);
        diesel::delete(
            games::table.filter(
                games::game_status
                    .eq(GameStatus::NotStarted.to_string())
                    .and(games::speed.ne(GameSpeed::Correspondence.to_string()))
                    .and(games::tournament_id.is_null())
                    .and(games::created_at.lt(cutoff)),
            ),
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    fn validate_options(options: &GamesQueryOptions) -> Result<GamesQueryOptions, DbError> {
        options
            .clone()
            .validate_all()
            .map_err(|errs| DbError::InvalidInput {
                info: errs
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
                error: String::new(),
            })
    }

    pub async fn get_rows_from_options(
        options: &GamesQueryOptions,
        conn: &mut DbConn<'_>,
    ) -> Result<(Vec<Game>, Option<BatchToken>, Option<i64>), DbError> {
        let prepared = Self::validate_options(options)?;
        let query = GameQueryBuilder::batch_query(&prepared).build();
        let records: Vec<Game> = query.select(games::all_columns).get_results(conn).await?;
        let total = if prepared.include_total {
            Some(
                GameQueryBuilder::count_query(&prepared)
                    .build()
                    .count()
                    .get_result(conn)
                    .await?,
            )
        } else {
            None
        };
        let next = Self::next_batch_token(&records, &prepared);
        Ok((records, next, total))
    }

    fn next_batch_token(rows: &[Game], options: &GamesQueryOptions) -> Option<BatchToken> {
        if rows.len() < options.batch_size {
            return None;
        }
        let last = rows.last()?;
        let primary_value = match options.sort.key {
            GameSortKey::Date => SortValue::UpdatedAt(last.updated_at),
            GameSortKey::Turns => SortValue::Turns(last.turn),
            GameSortKey::RatingAvg => {
                let (Some(white), Some(black)) = (last.white_rating, last.black_rating) else {
                    return None;
                };
                SortValue::RatingAvg((white + black) / 2.0)
            }
        };

        Some(BatchToken {
            sort: options.sort.clone(),
            primary_value,
            updated_at: last.updated_at,
            id: last.id,
        })
    }

    pub(crate) async fn start(
        &self,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        if self.finished || self.turn != 0 || self.game_status != GameStatus::NotStarted.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot start this game"),
            });
        }
        let new_timeout_at = compute_timeout_at(
            Some(effective_at),
            self.white_time_left,
            self.black_time_left,
            0,
            &self.time_mode,
            &GameStatus::InProgress.to_string(),
        );
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_status.eq(GameStatus::InProgress.to_string()),
                updated_at.eq(effective_at),
                last_interaction.eq(effective_at),
                timeout_at.eq(new_timeout_at),
            ))
            .get_result(conn)
            .await?)
    }

    pub fn str_time_left_for_player(&self, player: Uuid) -> String {
        if let Some(color) = self.user_color(player) {
            if let Ok(time) = self.time_left_duration(color) {
                if let Ok(mode) = TimeMode::from_str(&self.time_mode) {
                    return mode.time_remaining(time);
                }
            }
        }
        String::new()
    }

    pub async fn get_rating_history_for_player(
        player: Uuid,
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<GameRating>, DbError> {
        if matches!(game_speed, GameSpeed::Untimed) {
            return Ok(vec![]);
        }
        let games_preload = games::table
            .filter(rated.eq(true))
            .filter(finished.eq(true))
            .filter(speed.eq(game_speed.to_string()))
            .filter(white_id.eq(player).or(black_id.eq(player)))
            .filter(white_rating.is_not_null())
            .filter(black_rating.is_not_null())
            .filter(white_rating_change.is_not_null())
            .filter(black_rating_change.is_not_null())
            .filter(updated_at.is_not_null())
            .order(updated_at.asc())
            .select((
                white_id,
                white_rating.assume_not_null(),
                black_rating.assume_not_null(),
                white_rating_change.assume_not_null(),
                black_rating_change.assume_not_null(),
                updated_at,
            ))
            .load::<(Uuid, f64, f64, f64, f64, DateTime<Utc>)>(conn)
            .await?;

        let mut daily_ratings = Vec::<GameRating>::new();
        for (white, white_value, black_value, white_change, black_change, updated) in games_preload
        {
            let rating_value = if white == player {
                white_value + white_change
            } else {
                black_value + black_change
            };
            let day = updated
                .date_naive()
                .and_time(chrono::NaiveTime::MIN)
                .and_utc();
            let entry = GameRating {
                rating: rating_value,
                updated_at: day,
            };
            match daily_ratings.last_mut() {
                Some(previous) if previous.updated_at == day => *previous = entry,
                Some(_) | None => daily_ratings.push(entry),
            }
        }
        Ok(daily_ratings)
    }

    pub async fn count_needing_hash_backfill(conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(games::table
            .filter(games::history.ne(""))
            .filter(games::finished.eq(true))
            .filter(games::hashes.eq(Vec::<Option<i64>>::new()))
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn find_needing_hash_backfill(
        after_id: Option<Uuid>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let mut query = games::table
            .filter(games::history.ne(""))
            .filter(games::finished.eq(true))
            .filter(games::hashes.eq(Vec::<Option<i64>>::new()))
            .order(games::id.asc())
            .limit(limit)
            .into_boxed();
        if let Some(after) = after_id {
            query = query.filter(games::id.gt(after));
        }
        Ok(query.load(conn).await?)
    }

    pub async fn set_hashes(
        game_id: Uuid,
        new_hashes: Vec<Option<i64>>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::update(games::table.find(game_id))
            .set(games::hashes.eq(new_hashes))
            .execute(conn)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::conclusion_for;
    use hive_lib::{Color, GameResult, GameStatus};
    use shared_types::Conclusion;

    /// The final occurrence is the final ply (index 33 of 34), so the repetition ended the game.
    #[test]
    fn a_threefold_draw_concludes_as_a_repetition() {
        assert_eq!(
            conclusion_for(&GameStatus::Finished(GameResult::Draw), &[25, 29, 33], 34),
            Conclusion::Repetition
        );
    }

    /// The regression: replay populates `repeating_moves` without adjudicating, so a suppressed
    /// earlier repetition must not relabel a game that actually ended some other way.
    #[test]
    fn an_earlier_repetition_does_not_relabel_a_win() {
        assert_eq!(
            conclusion_for(
                &GameStatus::Finished(GameResult::Winner(Color::White)),
                &[25, 29, 33],
                60
            ),
            Conclusion::Board
        );
    }

    /// The subtler half: a draw can also be a Board conclusion (both Queens surrounded), so
    /// `Draw` plus a repetition in the list is still not enough.
    #[test]
    fn an_earlier_repetition_does_not_relabel_a_board_draw() {
        assert_eq!(
            conclusion_for(&GameStatus::Finished(GameResult::Draw), &[25, 29, 33], 60),
            Conclusion::Board
        );
    }

    #[test]
    fn a_draw_that_was_not_a_repetition_is_still_a_board_result() {
        assert_eq!(
            conclusion_for(&GameStatus::Finished(GameResult::Draw), &[], 40),
            Conclusion::Board
        );
        // Two occurrences is not a threefold, even ending on the repeated position.
        assert_eq!(
            conclusion_for(&GameStatus::Finished(GameResult::Draw), &[10, 14], 15),
            Conclusion::Board
        );
    }

    #[test]
    fn an_unfinished_game_has_no_conclusion() {
        assert_eq!(
            conclusion_for(&GameStatus::InProgress, &[25, 29, 33], 34),
            Conclusion::Unknown
        );
    }
}

#[cfg(test)]
mod tournament_game_tests {
    use super::*;
    use shared_types::{
        tournament::{RoundRobinGameId, SlotKey},
        CorrespondenceClock,
        RealtimeClock,
    };
    use std::num::NonZeroU32;

    fn round_robin_slot(clock: Clock) -> Slot {
        Slot {
            id: Uuid::new_v4(),
            key: SlotKey::RoundRobin {
                slot: RoundRobinGameId::new(0),
            },
            white: Uuid::new_v4(),
            black: Uuid::new_v4(),
            clock,
            resolution: None,
        }
    }

    #[test]
    fn realtime_tournament_game_waits_for_ready_handshake() {
        let slot = round_robin_slot(Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        }));
        let game = NewGame::for_tournament_slot(Uuid::new_v4(), &slot, Utc::now())
            .expect("valid realtime tournament game");

        assert_eq!(game.game_status, GameStatus::NotStarted.to_string());
        assert_eq!(game.game_start, GameStart::Ready.to_string());
        assert_eq!(game.time_mode, TimeMode::RealTime.to_string());
        assert_eq!(game.time_base, Some(300));
        assert_eq!(game.time_increment, Some(3));
        assert!(game.last_interaction.is_none());
        assert!(game.timeout_at.is_none());
    }

    #[test]
    fn correspondence_tournament_game_begins_when_released() {
        let slot = round_robin_slot(Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
            seconds_each: NonZeroU32::new(86_400).unwrap(),
        }));
        let now = Utc::now();
        let game = NewGame::for_tournament_slot(Uuid::new_v4(), &slot, now)
            .expect("valid correspondence tournament game");

        assert_eq!(game.game_status, GameStatus::InProgress.to_string());
        assert_eq!(game.game_start, GameStart::Immediate.to_string());
        assert_eq!(game.last_interaction, Some(now));
        assert_eq!(game.timeout_at, Some(now + ChronoDuration::days(1)));
    }

    #[test]
    fn correspondence_per_move_deadline_uses_the_increment_field() {
        let slot = round_robin_slot(Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(172_800).unwrap(),
        }));
        let now = Utc::now();
        let game = NewGame::for_tournament_slot(Uuid::new_v4(), &slot, now)
            .expect("valid correspondence tournament game");

        assert_eq!(game.time_base, None);
        assert_eq!(game.time_increment, Some(172_800));
        assert_eq!(game.timeout_at, Some(now + ChronoDuration::days(2)));
    }

    #[test]
    fn arena_tournament_game_uses_arena_ownership_shape() {
        let paired_at = Utc::now();
        let game = NewGame::for_arena(
            Uuid::new_v4(),
            0,
            Uuid::new_v4(),
            Uuid::new_v4(),
            RealtimeClock {
                base_seconds: NonZeroU32::new(180).unwrap(),
                increment_seconds: 2,
            },
            paired_at,
        )
        .expect("valid Arena tournament game");
        assert_eq!(game.tournament_slot_id, None);
        assert_eq!(game.arena_ordinal, Some(0));
    }
}
