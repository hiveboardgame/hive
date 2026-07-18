use crate::{
    db_error::DbError,
    helpers::GameQueryBuilder,
    models::{Challenge, GameFinishContext, GameHash, GameUser, Rating, Tournament},
    schema::{
        challenges::{self, nanoid as nanoid_field},
        games::{self, dsl::*, tournament_game_result},
        games_users,
    },
    DbConn,
};
use ::nanoid::nanoid;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use diesel::{prelude::*, ExpressionMethods, Insertable};
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{Color, GameControl, GameResult, GameStatus, GameType, History, State};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use shared_types::{
    BatchToken,
    ChallengeId,
    Conclusion,
    GameId,
    GameSortKey,
    GameSpeed,
    GameStart,
    GamesQueryOptions,
    SortValue,
    TimeMode,
    TournamentGameResult,
};
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

pub static NANOS_IN_SECOND: u64 = 1000000000_u64;

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
    Some(last + chrono::Duration::nanoseconds(running_nanos))
}

#[derive(Debug)]
struct TimeInfo {
    white_time_left: Option<i64>,
    black_time_left: Option<i64>,
    timed_out: bool,
    new_game_status: GameStatus,
}

impl TimeInfo {
    pub fn new(status: GameStatus) -> Self {
        Self {
            white_time_left: None,
            black_time_left: None,
            timed_out: false,
            new_game_status: status,
        }
    }
}

#[derive(Queryable, Debug, PartialEq)]
pub struct GameRatings {
    pub speed: String,
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_id: Uuid,
    pub black_id: Uuid,
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
    pub tournament_id: Option<Uuid>,
    pub tournament_game_result: String,
    pub game_start: String,
    pub move_times: Vec<Option<i64>>,
    pub timeout_at: Option<DateTime<Utc>>,
}

impl NewGame {
    pub fn new_from_tournament(white: Uuid, black: Uuid, tournament: &Tournament) -> Self {
        let (time_left, start, status, interaction) =
            match TimeMode::from_str(&tournament.time_mode).unwrap() {
                TimeMode::Untimed => unreachable!("Tournaments cannot be untimed"),
                TimeMode::RealTime => (
                    tournament
                        .time_base
                        .map(|base| (base as u64 * NANOS_IN_SECOND) as i64),
                    GameStart::Ready.to_string(),
                    GameStatus::NotStarted.to_string(),
                    None,
                ),
                TimeMode::Correspondence => (
                    match (tournament.time_base, tournament.time_increment) {
                        (Some(base), None) => Some((base as u64 * NANOS_IN_SECOND) as i64),
                        (None, Some(inc)) => Some((inc as u64 * NANOS_IN_SECOND) as i64),
                        _ => unreachable!(),
                    },
                    GameStart::Immediate.to_string(),
                    GameStatus::InProgress.to_string(),
                    Some(Utc::now()),
                ),
            };
        let initial_timeout_at = compute_timeout_at(
            interaction,
            time_left,
            time_left,
            0,
            &tournament.time_mode,
            &status,
        );

        Self {
            nanoid: nanoid!(12),
            current_player_id: white,
            black_id: black,
            finished: false,
            game_status: status,
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            time_mode: tournament.time_mode.to_owned(),
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            last_interaction: interaction,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment)
                .to_string(),
            hashes: vec![],
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: Some(tournament.id),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: start,
            move_times: vec![],
            timeout_at: initial_timeout_at,
        }
    }

    pub fn new(white: Uuid, black: Uuid, challenge: &Challenge) -> Result<Self, DbError> {
        if white == black {
            return Err(DbError::InvalidInput {
                info: "You can't play here with yourself.".to_string(),
                error: String::new(),
            });
        }

        let time_left = match TimeMode::from_str(&challenge.time_mode).unwrap() {
            TimeMode::Untimed => None,
            TimeMode::RealTime => challenge
                .time_base
                .map(|base| (base as u64 * NANOS_IN_SECOND) as i64),
            TimeMode::Correspondence => match (challenge.time_base, challenge.time_increment) {
                (Some(base), None) => Some((base as u64 * NANOS_IN_SECOND) as i64),
                (None, Some(inc)) => Some((inc as u64 * NANOS_IN_SECOND) as i64),
                _ => unreachable!(),
            },
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
            speed: GameSpeed::from_base_increment(challenge.time_base, challenge.time_increment)
                .to_string(),
            hashes: vec![],
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: vec![],
            timeout_at: None,
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

    pub async fn check_time(&self, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        let game_id = self.id;
        conn.transaction::<_, DbError, _>(async move |tc| {
            // Stale Game values can race here; only the post-lock row may apply ratings.
            let game: Game = games::table.find(game_id).for_update().first(tc).await?;
            if game.finished {
                return Ok(game);
            }
            if let Some(timed_out_color) = game.timed_out_color()? {
                return game.finish_timeout(timed_out_color, tc).await;
            }
            Ok(game)
        })
        .await
    }

    fn stale_game_action_error() -> DbError {
        DbError::InvalidAction {
            info: String::from("Game changed before the action could be applied"),
        }
    }

    fn guarded_update_result(update: Result<Game, diesel::result::Error>) -> Result<Game, DbError> {
        match update {
            Ok(game) => Ok(game),
            Err(diesel::result::Error::NotFound) => Err(Self::stale_game_action_error()),
            Err(err) => Err(err.into()),
        }
    }

    async fn locked_unfinished(game_id: Uuid, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        let game: Game = games::table.find(game_id).for_update().first(conn).await?;
        if game.finished {
            return Err(DbError::GameIsOver);
        }
        Ok(game)
    }

    fn timed_out_color(&self) -> Result<Option<Color>, DbError> {
        if self.finished || TimeMode::from_str(&self.time_mode)? == TimeMode::Untimed {
            return Ok(None);
        }
        if GameStatus::NotStarted.to_string() == self.game_status {
            return Ok(None);
        }

        let Some(last_seen) = self.last_interaction else {
            todo!("Well this is not good and needs a better error message");
        };

        let active_color = if self.turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        };
        let time_left = self.time_left_duration(active_color)?;
        if let Ok(time_passed) = Utc::now().signed_duration_since(last_seen).to_std() {
            if time_left > time_passed {
                return Ok(None);
            }
        }
        Ok(Some(active_color))
    }

    async fn finish_timeout(
        &self,
        timed_out_color: Color,
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
                games::updated_at.eq(Utc::now()),
                games::white_time_left.eq(new_white_time_left),
                games::black_time_left.eq(new_black_time_left),
                games::conclusion.eq(Conclusion::Timeout.to_string()),
                games::timeout_at.eq(CLEAR_TIMEOUT_AT),
            ))
            .get_result(conn)
            .await?;
        let ctx = GameFinishContext::from_finished_game(&game);
        if let Ok(state) = State::new_from_str(&game.history, &game.game_type) {
            GameHash::insert_for_game(game.id, &state.hashes, &state.history.moves, &ctx, conn)
                .await?;
        }
        Ok(game)
    }

    async fn finish_game_control(
        &self,
        game_control: GameControl,
        result: GameResult,
        final_conclusion: Conclusion,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        let (new_white_time_left, new_black_time_left) = match TimeMode::from_str(&self.time_mode)?
        {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left()?,
        };
        if new_white_time_left == Some(0) {
            return self.finish_timeout(Color::White, conn).await;
        }
        if new_black_time_left == Some(0) {
            return self.finish_timeout(Color::Black, conn).await;
        }
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
                games::updated_at.eq(Utc::now()),
                games::white_time_left.eq(new_white_time_left),
                games::black_time_left.eq(new_black_time_left),
                games::conclusion.eq(final_conclusion.to_string()),
                games::timeout_at.eq(CLEAR_TIMEOUT_AT),
            ))
            .get_result(conn)
            .await?;
        let ctx = GameFinishContext::from_finished_game(&game);
        if let Ok(state) = State::new_from_str(&game.history, &game.game_type) {
            GameHash::insert_for_game(game.id, &state.hashes, &state.history.moves, &ctx, conn)
                .await?;
        }
        Ok(game)
    }

    fn time_left_duration(&self, color: Color) -> Result<Duration, DbError> {
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

    fn time_increment_duration(&self) -> Result<Duration, DbError> {
        if let Some(increment) = self.time_increment {
            Ok(Duration::from_secs(increment as u64))
        } else {
            Err(DbError::TimeNotFound {
                reason: String::from("Could not find time_increment"),
            })
        }
    }

    fn calculate_time_left(&self) -> Result<(Option<i64>, Option<i64>), DbError> {
        let mut time_left = self.time_left_duration(if self.turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        })?;
        let (mut black_time, mut white_time) = (self.black_time_left, self.white_time_left);
        if let Some(last) = self.last_interaction {
            let time_passed = Utc::now().signed_duration_since(last).to_std().unwrap();
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

    fn calculate_time_left_add_increment(
        &self,
        shutout: bool,
        comp: f64,
    ) -> Result<(Option<i64>, Option<i64>), DbError> {
        let (mut white_time, mut black_time) = self.calculate_time_left()?;
        if let (Some(w), Some(b)) = (white_time, black_time) {
            if w == 0 || b == 0 {
                return Ok((white_time, black_time));
            }
        }
        let comp = (comp * 1_000_000_000.0) as i64;
        let increment = self.time_increment_duration()?.as_nanos() as i64;
        if self.turn % 2 == 0 {
            white_time = white_time.map(|time| time + increment + comp);
        } else {
            black_time = black_time.map(|time| time + increment + comp);
        };
        if shutout {
            if self.turn % 2 == 0 {
                black_time = black_time.map(|time| time + increment);
            } else {
                white_time = white_time.map(|time| time + increment);
            };
        };

        Ok((white_time, black_time))
    }

    fn get_time_info(&self, state: &State, comp: f64) -> Result<TimeInfo, DbError> {
        match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => Ok(TimeInfo::new(state.game_status.clone())),
            TimeMode::RealTime => self.get_realtime_time_info(state, comp),
            TimeMode::Correspondence => self.get_correspondence_time_info(state),
        }
    }

    fn get_realtime_time_info(&self, state: &State, comp: f64) -> Result<TimeInfo, DbError> {
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
                self.calculate_time_left_add_increment(state.history.last_move_is_pass(), comp)?;
            if self.turn % 2 == 0 {
                if time_info.white_time_left == Some(0) {
                    time_info.timed_out = true;
                    time_info.new_game_status =
                        GameStatus::Finished(GameResult::Winner(Color::Black));
                }
            } else if time_info.black_time_left == Some(0) {
                time_info.timed_out = true;
                time_info.new_game_status = GameStatus::Finished(GameResult::Winner(Color::White));
            }
        }
        Ok(time_info)
    }

    fn get_correspondence_time_info(&self, state: &State) -> Result<TimeInfo, DbError> {
        let mut time_info = TimeInfo::new(state.game_status.clone());
        if self.turn < 2 && self.game_start == GameStart::Moves.to_string() {
            if self.turn == 0 {
                time_info.new_game_status = GameStatus::NotStarted;
            };
            time_info.white_time_left = self.white_time_left;
            time_info.black_time_left = self.black_time_left;
        } else {
            (time_info.white_time_left, time_info.black_time_left) = self.calculate_time_left()?;
            if self.turn % 2 == 0 {
                if time_info.white_time_left == Some(0) {
                    time_info.timed_out = true;
                    time_info.new_game_status =
                        GameStatus::Finished(GameResult::Winner(Color::Black));
                } else {
                    match (self.time_increment, self.time_base) {
                        (Some(inc), None) => {
                            time_info.white_time_left = Some((inc as u64 * NANOS_IN_SECOND) as i64);
                            if state.history.last_move_is_pass() {
                                time_info.black_time_left =
                                    Some((inc as u64 * NANOS_IN_SECOND) as i64);
                            }
                        }
                        (None, Some(_)) => {}
                        _ => unreachable!(),
                    }
                }
            } else if time_info.black_time_left == Some(0) {
                time_info.timed_out = true;
                time_info.new_game_status = GameStatus::Finished(GameResult::Winner(Color::White));
            } else {
                match (self.time_increment, self.time_base) {
                    (Some(inc), None) => {
                        time_info.black_time_left = Some((inc as u64 * NANOS_IN_SECOND) as i64);
                        if state.history.last_move_is_pass() {
                            time_info.white_time_left = Some((inc as u64 * NANOS_IN_SECOND) as i64);
                        }
                    }
                    (None, Some(_)) => {}
                    _ => unreachable!(),
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

    pub async fn update_gamestate(
        &self,
        state: &State,
        comp: f64,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let time_info = self.get_time_info(state, comp)?;
        let new_history = state
            .history
            .moves
            .iter()
            .map(|(piece, destination)| format!("{piece} {destination};"))
            .collect::<Vec<String>>()
            .join("");

        let game_control_string = if self.has_unanswered_game_control() {
            let gc = match self.last_game_control() {
                Some(GameControl::TakebackRequest(color)) => {
                    GameControl::TakebackReject(color.opposite_color())
                }
                Some(GameControl::DrawOffer(color)) => {
                    GameControl::DrawReject(color.opposite_color())
                }
                _ => unreachable!(),
            };
            format!("{}. {gc};", self.turn)
        } else {
            String::new()
        };

        let mut new_conclusion = match &time_info.new_game_status {
            GameStatus::Finished(GameResult::Draw | GameResult::Winner(_)) => Conclusion::Board,
            _ => Conclusion::Unknown,
        };
        if state.repeating_moves.len() > 2 {
            new_conclusion = Conclusion::Repetition;
        }

        let next_player = if state.turn.is_multiple_of(2) {
            self.white_id
        } else {
            self.black_id
        };

        let new_move_times = self.get_move_times(&time_info, state);
        let new_hashes: Vec<Option<i64>> = state.hashes.iter().map(|h| Some(*h as i64)).collect();
        let raw_hashes: Vec<u64> = state.hashes.clone();
        let new_moves = state.history.moves.clone();

        if time_info.timed_out {
            // Timeout supersedes the in-flight move and any implicit control rejection it carried.
            return self.check_time(conn).await;
        }

        if let GameStatus::Finished(game_result) = time_info.new_game_status.clone() {
            if let GameResult::Unknown = game_result {
                panic!("GameResult is unknown but the game is over");
            };
            let game_id = self.id;
            let expected_turn = self.turn;
            let expected_history = self.history.clone();
            let new_turn = state.turn as i32;
            let new_white_time_left = time_info.white_time_left;
            let new_black_time_left = time_info.black_time_left;
            return conn
                .transaction::<_, DbError, _>(async move |tc| {
                    let game: Game = games::table.find(game_id).for_update().first(tc).await?;
                    if game.finished {
                        return Err(DbError::GameIsOver);
                    }
                    if game.turn != expected_turn || game.history != expected_history {
                        return Err(Self::stale_game_action_error());
                    }
                    let tgr = TournamentGameResult::new(&game_result);
                    let new_game_status = GameStatus::Finished(game_result.clone());
                    let (
                        white_rating_before,
                        black_rating_before,
                        new_white_rating_change,
                        new_black_rating_change,
                    ) = Rating::update(
                        game.rated,
                        game.speed.clone(),
                        game.white_id,
                        game.black_id,
                        game_result,
                        tc,
                    )
                    .await?;
                    let updated_game: Game = diesel::update(games::table.find(game.id))
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
                            games::updated_at.eq(Utc::now()),
                            games::white_time_left.eq(new_white_time_left),
                            games::black_time_left.eq(new_black_time_left),
                            games::last_interaction.eq(Some(Utc::now())),
                            games::move_times.eq(new_move_times),
                            games::hashes.eq(&new_hashes),
                            games::conclusion.eq(new_conclusion.to_string()),
                            games::timeout_at.eq(CLEAR_TIMEOUT_AT),
                        ))
                        .get_result(tc)
                        .await?;
                    let ctx = GameFinishContext::from_finished_game(&updated_game);
                    GameHash::insert_for_game(updated_game.id, &raw_hashes, &new_moves, &ctx, tc)
                        .await?;
                    Ok(updated_game)
                })
                .await;
        }

        // Moves intentionally guard board progress only. Concurrent control-log
        // writes survive, and a move must not reject an offer the mover did not see.
        let now = Utc::now();
        let new_turn = state.turn as i32;
        let new_status_str = time_info.new_game_status.to_string();
        let new_timeout_at = compute_timeout_at(
            Some(now),
            time_info.white_time_left,
            time_info.black_time_left,
            new_turn,
            &self.time_mode,
            &new_status_str,
        );
        let update = diesel::update(
            games::table
                .find(self.id)
                .filter(games::finished.eq(false))
                .filter(games::turn.eq(self.turn))
                .filter(games::history.eq(self.history.clone())),
        )
        .set((
            history.eq(new_history),
            current_player_id.eq(next_player),
            turn.eq(new_turn),
            game_status.eq(new_status_str),
            game_control_history.eq(game_control_history.concat(game_control_string)),
            updated_at.eq(now),
            white_time_left.eq(time_info.white_time_left),
            black_time_left.eq(time_info.black_time_left),
            move_times.eq(new_move_times),
            last_interaction.eq(Some(now)),
            timeout_at.eq(new_timeout_at),
            hashes.eq(new_hashes),
        ))
        .get_result(conn)
        .await;
        Self::guarded_update_result(update)
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

    pub fn has_unanswered_game_control(&self) -> bool {
        self.last_game_control().is_some_and(|gc| {
            matches!(
                gc,
                GameControl::TakebackRequest(_) | GameControl::DrawOffer(_)
            )
        })
    }

    pub fn last_game_control(&self) -> Option<GameControl> {
        if let Some(last) = self.game_control_history.split_terminator(';').next_back() {
            if let Some(gc) = last.split(' ').next_back() {
                return Some(
                    GameControl::from_str(gc)
                        .expect("Could not get GameControl from game_control_history"),
                );
            }
        }
        None
    }

    pub async fn write_game_control(
        &self,
        game_control: &GameControl,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        let update = diesel::update(
            games::table
                .find(self.id)
                .filter(games::finished.eq(false))
                .filter(games::turn.eq(self.turn))
                .filter(games::history.eq(self.history.clone()))
                .filter(games::game_control_history.eq(self.game_control_history.clone())),
        )
        .set((
            game_control_history.eq(game_control_history.concat(game_control_string)),
            updated_at.eq(Utc::now()),
        ))
        .get_result(conn)
        .await;
        Self::guarded_update_result(update)
    }

    fn get_takeback_time_correspondence(&self, popped: i32) -> (Option<i64>, Option<i64>) {
        // For TotalTimeEach increment: None, base: Some
        if self.time_increment.is_none() {
            return self.get_takeback_time_realtime(popped);
        }

        // For DaysPerMove increment: Some and base: None
        let mut black_time = self.black_time_left;
        let mut white_time = self.white_time_left;

        if self.turn % 2 == 0 {
            black_time = self
                .time_increment
                .map(|t| t as i64 * NANOS_IN_SECOND as i64);
        } else {
            white_time = self
                .time_increment
                .map(|t| t as i64 * NANOS_IN_SECOND as i64);
        }

        if popped == 2 {
            if self.turn % 2 == 0 {
                white_time = self
                    .time_increment
                    .map(|t| t as i64 * NANOS_IN_SECOND as i64);
            } else {
                black_time = self
                    .time_increment
                    .map(|t| t as i64 * NANOS_IN_SECOND as i64);
            }
        }

        (white_time, black_time)
    }

    fn get_takeback_time_realtime(&self, popped: i32) -> (Option<i64>, Option<i64>) {
        let past_turn = self.turn - popped;
        let mut times = self.move_times.clone();
        let mut black_time = self.black_time_left;
        let mut white_time = self.white_time_left;

        if self.turn % 2 == 0 {
            black_time = times.pop().unwrap_or(Some(0));
        } else {
            white_time = times.pop().unwrap_or(Some(0));
        }

        if popped == 2 {
            if self.turn % 2 == 0 {
                white_time = times.pop().unwrap_or(Some(0));
            } else {
                black_time = times.pop().unwrap_or(Some(0));
            }
        }

        if past_turn > 1 {
            if self.turn % 2 == 0 {
                black_time = Some(
                    black_time.unwrap_or(0)
                        - self.time_increment.unwrap_or(0) as i64 * NANOS_IN_SECOND as i64,
                );
            } else {
                white_time = Some(
                    white_time.unwrap_or(0)
                        - self.time_increment.unwrap_or(0) as i64 * NANOS_IN_SECOND as i64,
                );
            }
            if popped == 2 {
                if self.turn % 2 == 0 {
                    white_time = Some(
                        white_time.unwrap_or(0)
                            - self.time_increment.unwrap_or(0) as i64 * NANOS_IN_SECOND as i64,
                    );
                } else {
                    black_time = Some(
                        black_time.unwrap_or(0)
                            - self.time_increment.unwrap_or(0) as i64 * NANOS_IN_SECOND as i64,
                    );
                }
            }
        }

        (white_time, black_time)
    }

    fn get_takeback_time(&self, popped: i32) -> Result<(Option<i64>, Option<i64>), DbError> {
        match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => Ok((None, None)),
            TimeMode::Correspondence => Ok(self.get_takeback_time_correspondence(popped)),
            TimeMode::RealTime => Ok(self.get_takeback_time_realtime(popped)),
        }
    }

    pub async fn accept_takeback(
        &self,
        game_control: &GameControl,
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

        let his = History::new_from_str(&new_history).map_err(|e| DbError::InvalidInput {
            info: String::from("Could not recover History from history string."),
            error: e.to_string(),
        })?;
        let state = State::new_from_history(&his).map_err(|e| DbError::InvalidInput {
            info: String::from("Could not recover State from History."),
            error: e.to_string(),
        })?;
        let new_game_status = state.game_status.to_string();
        let next_player = if self.current_player_id == self.black_id {
            self.white_id
        } else {
            self.black_id
        };
        let new_turn = self.turn - popped;
        let now = Utc::now();
        // None on takeback to turn 0, since the rebuilt status is NotStarted.
        let new_timeout_at = compute_timeout_at(
            Some(now),
            white_time,
            black_time,
            new_turn,
            &self.time_mode,
            &new_game_status,
        );

        let update = diesel::update(
            games::table
                .find(self.id)
                .filter(games::finished.eq(false))
                .filter(games::turn.eq(self.turn))
                .filter(games::history.eq(self.history.clone()))
                .filter(games::game_control_history.eq(self.game_control_history.clone())),
        )
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
        .await;
        Self::guarded_update_result(update)
    }

    pub async fn resign(
        &self,
        game_control: &GameControl,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control = *game_control;
        let game = Self::locked_unfinished(self.id, conn).await?;
        if let Some(timed_out_color) = game.timed_out_color()? {
            return game.finish_timeout(timed_out_color, conn).await;
        }

        let result = GameResult::Winner(game_control.color().opposite_color());
        game.finish_game_control(game_control, result, Conclusion::Resigned, conn)
            .await
    }

    pub async fn accept_draw(
        &self,
        game_control: &GameControl,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control = *game_control;
        let game = Self::locked_unfinished(self.id, conn).await?;
        if let Some(timed_out_color) = game.timed_out_color()? {
            return game.finish_timeout(timed_out_color, conn).await;
        }

        let expected_offer = GameControl::DrawOffer(game_control.color().opposite_color());
        // Re-check under the row lock: any intervening control makes this accept stale.
        if game.last_game_control() != Some(expected_offer) {
            return Err(Self::stale_game_action_error());
        }

        game.finish_game_control(game_control, GameResult::Draw, Conclusion::Draw, conn)
            .await
    }

    pub async fn set_status(
        &self,
        status: GameStatus,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_status.eq(status.to_string()),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn find_by_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        let game: Game = games::table.find(uuid).first(conn).await?;
        if !game.finished && TimeMode::from_str(&game.time_mode)? != TimeMode::Untimed {
            game.check_time(conn).await
        } else {
            Ok(game)
        }
    }

    pub async fn find_by_game_id(game_id: &GameId, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
        let game: Game = games::table
            .filter(nanoid.eq(game_id.0.clone()))
            .first(conn)
            .await?;
        if !game.finished && TimeMode::from_str(&game.time_mode)? != TimeMode::Untimed {
            game.check_time(conn).await
        } else {
            Ok(game)
        }
    }

    pub async fn find_by_game_ids(
        game_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let found_games: Vec<Game> = games::table.filter(id.eq_any(game_ids)).load(conn).await?;

        let mut checked_games = Vec::new();
        for game in found_games {
            if !game.finished && TimeMode::from_str(&game.time_mode)? != TimeMode::Untimed {
                checked_games.push(game.check_time(conn).await?);
            } else {
                checked_games.push(game);
            }
        }
        Ok(checked_games)
    }

    /// Best-effort batched lookup used by the websocket heartbeat. Rows whose
    /// `time_mode` fails to parse or whose `check_time` returns an error are
    /// silently dropped from the result rather than aborting the whole batch
    /// — one bad row must not stall heartbeats for every other active game.
    /// The outer DB load is still strict; only per-row processing is tolerant.
    pub async fn find_by_nanoids(
        game_ids: &[GameId],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let nanoids: Vec<String> = game_ids.iter().map(|g| g.0.clone()).collect();
        let found_games: Vec<Game> = games::table
            .filter(nanoid.eq_any(&nanoids))
            .load(conn)
            .await?;

        let mut checked_games = Vec::new();
        for game in found_games {
            if game.finished {
                checked_games.push(game);
                continue;
            }
            let Ok(mode) = TimeMode::from_str(&game.time_mode) else {
                continue;
            };
            if mode == TimeMode::Untimed {
                checked_games.push(game);
                continue;
            }
            if let Ok(checked) = game.check_time(conn).await {
                checked_games.push(checked);
            }
        }
        Ok(checked_games)
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
            .filter(games::timeout_at.is_not_null())
            .filter(games::timeout_at.le(as_of))
            .order(games::timeout_at.asc())
            .limit(limit)
            .load(conn)
            .await?)
    }

    pub async fn delete(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
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

    pub async fn get_ongoing_ids_for_tournament(
        tournament_id_: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<String>, DbError> {
        Ok(games::table
            .filter(
                games::tournament_id
                    .eq(tournament_id_)
                    .and(games::finished.eq(false)),
            )
            .select(games::nanoid)
            .get_results(conn)
            .await?)
    }

    pub async fn get_ongoing_ids_for_tournament_by_user(
        tournament_id_: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<String>, DbError> {
        Ok(games::table
            .filter(
                games::tournament_id.eq(tournament_id_).and(
                    games::finished
                        .eq(false)
                        .and(games::white_id.eq(user_id).or(games::black_id.eq(user_id))),
                ),
            )
            .select(games::nanoid)
            .get_results(conn)
            .await?)
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

    pub async fn adjudicate_tournament_result(
        &self,
        user_id: &Uuid,
        new_result: &TournamentGameResult,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        if !(matches!(
            Conclusion::from_str(&self.conclusion),
            Ok(Conclusion::Committee) | Ok(Conclusion::Unknown) | Ok(Conclusion::Forfeit)
        ) && self.turn == 0
            && self.history.is_empty()
            && self.game_start == GameStart::Ready.to_string()
            && matches!(
                GameStatus::from_str(&self.game_status),
                Ok(GameStatus::NotStarted) | Ok(GameStatus::Adjudicated)
            ))
        {
            return Err(DbError::InvalidAction {
                info: String::from("You cannot adjudicate a game that has already started"),
            });
        }

        let tid = self.tournament_id.ok_or_else(|| DbError::InvalidAction {
            info: String::from("Not a tournament game"),
        })?;
        let tournament = Tournament::find(tid, conn).await?;
        tournament
            .ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;

        self.update_tournament_result(new_result, conn).await
    }

    pub(crate) async fn assign_tournament_result(
        &self,
        new_result: &TournamentGameResult,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        if self.tournament_id.is_none() {
            return Err(DbError::InvalidAction {
                info: String::from("Not a tournament game"),
            });
        }
        self.update_tournament_result(new_result, conn).await
    }

    async fn update_tournament_result(
        &self,
        new_result: &TournamentGameResult,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let (con, status, fin, new_last_interaction) = match new_result {
            TournamentGameResult::DoubeForfeit => (
                Conclusion::Forfeit,
                GameStatus::Adjudicated,
                true,
                Some(Utc::now()),
            ),
            TournamentGameResult::Unknown => {
                (Conclusion::Unknown, GameStatus::NotStarted, false, None)
            }
            _ => (
                Conclusion::Committee,
                GameStatus::Adjudicated,
                true,
                Some(Utc::now()),
            ),
        };
        // Every branch ends in a no-clock status, so timeout_at is None.
        let game = diesel::update(games::table.find(self.id))
            .set((
                finished.eq(fin),
                conclusion.eq(con.to_string()),
                game_status.eq(status.to_string()),
                tournament_game_result.eq(new_result.to_string()),
                updated_at.eq(Utc::now()),
                last_interaction.eq(new_last_interaction),
                timeout_at.eq(CLEAR_TIMEOUT_AT),
            ))
            .get_result(conn)
            .await?;
        Ok(game)
    }

    pub async fn start(&self, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        if self.finished || self.turn > 0 || self.game_status != GameStatus::NotStarted.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot start this game"),
            });
        }
        let now = Utc::now();
        let new_timeout_at = compute_timeout_at(
            Some(now),
            self.white_time_left,
            self.black_time_left,
            0,
            &self.time_mode,
            &GameStatus::InProgress.to_string(),
        );
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_status.eq(GameStatus::InProgress.to_string()),
                updated_at.eq(now),
                last_interaction.eq(now),
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

    pub fn not_current_player_id(&self) -> Uuid {
        if self.black_id == self.current_player_id {
            return self.white_id;
        }
        self.black_id
    }

    pub async fn get_rating_history_for_player(
        player: Uuid,
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<GameRatings>, DbError> {
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
            .load::<Game>(conn)
            .await?;

        Ok(games_preload
            .into_iter()
            .chunk_by(|game| {
                let utc = game.updated_at.with_timezone(&Utc);
                (utc.year(), utc.month(), utc.day())
            })
            .into_iter()
            .filter_map(|((_y, _m, _d), group)| {
                let last = group.last()?;
                let utc_day = last.updated_at.with_timezone(&Utc).date_naive();
                let utc_datetime = Utc
                    .from_local_datetime(&utc_day.and_hms_opt(0, 0, 0)?)
                    .single()?;
                Some(GameRatings {
                    speed: last.speed.clone(),
                    white_rating: last
                        .white_rating
                        .map(|r| r + last.white_rating_change.unwrap_or(0.0)),
                    black_rating: last
                        .black_rating
                        .map(|r| r + last.black_rating_change.unwrap_or(0.0)),
                    white_id: last.white_id,
                    black_id: last.black_id,
                    updated_at: utc_datetime,
                })
            })
            .collect())
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
