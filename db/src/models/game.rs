use crate::{
    db_error::DbError,
    models::{Challenge, GameUser, Rating, Tournament},
    schema::{
        challenges::{self, nanoid as nanoid_field},
        games::{self, dsl::*, tournament_game_result},
        games_users,
        users::{self},
    },
    DbConn,
};
use ::nanoid::nanoid;
use chrono::{DateTime, Utc};
use diesel::{prelude::*, ExpressionMethods, Insertable};
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameControl, GameResult, GameStatus, GameType, History, State};
use serde::{Deserialize, Serialize};
use shared_types::{
    ChallengeId, Conclusion, GameId, GameProgress, GameSpeed, GameStart, GamesQueryOptions,
    ResultType, TimeMode, TournamentGameResult,
};
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

pub static NANOS_IN_SECOND: u64 = 1000000000_u64;

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
        }
    }

    pub fn new(white: Uuid, black: Uuid, challenge: &Challenge) -> Self {
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

        Self {
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
        }
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
}

impl Game {
    pub fn hashes(&self) -> Vec<u64> {
        // WARN: @leex reimplement this
        //self.hashes.iter().map(|i| *i as u64).collect::<Vec<u64>>()
        vec![]
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
        let white = self.white_time_left_duration()?;
        let black = self.black_time_left_duration()?;
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
        if self.time_mode == "Unlimited" || self.finished {
            return Ok(self.clone());
        }
        let time_left = if self.turn % 2 == 0 {
            self.white_time_left_duration()?
        } else {
            self.black_time_left_duration()?
        };
        if GameStatus::NotStarted.to_string() == self.game_status {
            return Ok(self.clone());
        }
        if let Some(last) = self.last_interaction {
            if let Ok(time_passed) = Utc::now().signed_duration_since(last).to_std() {
                if time_left > time_passed {
                    return Ok(self.clone());
                }
            }
            let (white_time, black_time, game_result) = if self.turn % 2 == 0 {
                (
                    Some(0_i64),
                    self.black_time_left,
                    GameResult::Winner(Color::Black),
                )
            } else {
                (
                    self.white_time_left,
                    Some(0),
                    GameResult::Winner(Color::White),
                )
            };
            let tgr = TournamentGameResult::new(&game_result);
            let new_game_status = GameStatus::Finished(game_result.clone());
            let (w_rating, b_rating, w_change, b_change) = Rating::update(
                self.rated,
                self.speed.clone(),
                self.white_id,
                self.black_id,
                game_result,
                conn,
            )
            .await?;
            let game = diesel::update(games::table.find(self.id))
                .set((
                    finished.eq(true),
                    tournament_game_result.eq(tgr.to_string()),
                    game_status.eq(new_game_status.to_string()),
                    white_rating.eq(w_rating),
                    black_rating.eq(b_rating),
                    white_rating_change.eq(w_change),
                    black_rating_change.eq(b_change),
                    updated_at.eq(Utc::now()),
                    white_time_left.eq(white_time),
                    black_time_left.eq(black_time),
                    conclusion.eq(Conclusion::Timeout.to_string()),
                ))
                .get_result(conn)
                .await?;
            Ok(game)
        } else {
            todo!("Well this is not good and needs a better error message");
        }
    }

    fn white_time_left_duration(&self) -> Result<Duration, DbError> {
        if let Some(white_time) = self.white_time_left {
            Ok(Duration::from_nanos(white_time as u64))
        } else {
            Err(DbError::TimeNotFound {
                reason: String::from("Could not find white_time"),
            })
        }
    }

    fn black_time_left_duration(&self) -> Result<Duration, DbError> {
        if let Some(black_time) = self.black_time_left {
            Ok(Duration::from_nanos(black_time as u64))
        } else {
            Err(DbError::TimeNotFound {
                reason: String::from("Could not find black_time"),
            })
        }
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
        let mut time_left = if self.turn % 2 == 0 {
            self.white_time_left_duration()?
        } else {
            self.black_time_left_duration()?
        };
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
                if state.turn % 2 == 0 {
                    new_move_times.push(time_info.black_time_left);
                } else {
                    new_move_times.push(time_info.white_time_left);
                }
            } else {
                // A shutout has happened, so state.turn was incremented twice so the "previous/not
                // shutout" player's time has to be added first. Note that we need to do it the
                // other way round than in if it's not a shutout
                if state.turn % 2 == 0 {
                    new_move_times.push(time_info.white_time_left);
                } else {
                    new_move_times.push(time_info.black_time_left);
                }
                // Now the shutout player's time can be added
                if state.turn % 2 == 0 {
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
        let mut new_history = state
            .history
            .moves
            .iter()
            .map(|(piece, destination)| format!("{piece} {destination};"))
            .collect::<Vec<String>>()
            .join("");

        let mut game_control_string = String::new();
        if self.has_unanswered_game_control() {
            let gc = match self.last_game_control() {
                Some(GameControl::TakebackRequest(color)) => {
                    GameControl::TakebackReject(color.opposite_color())
                }
                Some(GameControl::DrawOffer(color)) => {
                    GameControl::DrawReject(color.opposite_color())
                }
                _ => unreachable!(),
            };
            game_control_string = format!("{}. {gc};", self.turn);
        }

        let mut new_conclusion = Conclusion::Unknown;
        match time_info.new_game_status {
            GameStatus::Finished(GameResult::Draw) => new_conclusion = Conclusion::Board,
            GameStatus::Finished(GameResult::Winner(_)) => new_conclusion = Conclusion::Board,
            _ => {}
        }
        if state.repeating_moves.len() > 2 {
            new_conclusion = Conclusion::Repetition;
        }

        let next_player = if state.turn % 2 == 0 {
            self.white_id
        } else {
            self.black_id
        };

        let new_move_times = self.get_move_times(&time_info, state);

        if let GameStatus::Finished(game_result) = time_info.new_game_status.clone() {
            if let GameResult::Unknown = game_result {
                panic!("GameResult is unknown but the game is over");
            };
            let tgr = TournamentGameResult::new(&game_result);
            let (w_rating, b_rating, w_change, b_change) = Rating::update(
                self.rated,
                self.speed.clone(),
                self.white_id,
                self.black_id,
                game_result,
                conn,
            )
            .await?;

            let new_turn = if time_info.timed_out {
                self.turn
            } else {
                state.turn as i32
            };

            if time_info.timed_out {
                new_conclusion = Conclusion::Timeout;
                new_history.clone_from(&self.history);
            }

            let game = diesel::update(games::table.find(self.id))
                .set((
                    history.eq(new_history),
                    current_player_id.eq(next_player),
                    turn.eq(new_turn),
                    finished.eq(true),
                    tournament_game_result.eq(tgr.to_string()),
                    game_status.eq(time_info.new_game_status.to_string()),
                    game_control_history.eq(game_control_history.concat(game_control_string)),
                    white_rating.eq(w_rating),
                    black_rating.eq(b_rating),
                    white_rating_change.eq(w_change),
                    black_rating_change.eq(b_change),
                    updated_at.eq(Utc::now()),
                    white_time_left.eq(time_info.white_time_left),
                    black_time_left.eq(time_info.black_time_left),
                    last_interaction.eq(Some(Utc::now())),
                    move_times.eq(new_move_times),
                    conclusion.eq(new_conclusion.to_string()),
                ))
                .get_result(conn)
                .await?;
            Ok(game)
        } else {
            let game = diesel::update(games::table.find(self.id))
                .set((
                    history.eq(new_history),
                    current_player_id.eq(next_player),
                    turn.eq(state.turn as i32),
                    game_status.eq(time_info.new_game_status.to_string()),
                    game_control_history.eq(game_control_history.concat(game_control_string)),
                    updated_at.eq(Utc::now()),
                    white_time_left.eq(time_info.white_time_left),
                    black_time_left.eq(time_info.black_time_left),
                    move_times.eq(new_move_times),
                    last_interaction.eq(Some(Utc::now())),
                ))
                .get_result(conn)
                .await?;
            Ok(game)
        }
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
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
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

        Ok(diesel::update(games::table.find(self.id))
            .set((
                current_player_id.eq(next_player),
                history.eq(new_history),
                turn.eq(turn - popped),
                game_status.eq(new_game_status),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(Utc::now()),
                last_interaction.eq(Utc::now()),
                move_times.eq(new_move_times),
                white_time_left.eq(white_time),
                black_time_left.eq(black_time),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn resign(
        &self,
        game_control: &GameControl,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);

        let winner_color = game_control.color().opposite_color();
        let new_game_status = GameStatus::Finished(GameResult::Winner(winner_color));

        let (white_time, black_time) = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left()?,
        };
        if white_time == Some(0) || black_time == Some(0) {
            return self.check_time(conn).await;
        }
        let ((w_rating, b_rating, w_change, b_change), tgr) = match new_game_status.clone() {
            GameStatus::Finished(game_result) => (
                Rating::update(
                    self.rated,
                    self.speed.clone(),
                    self.white_id,
                    self.black_id,
                    game_result.clone(),
                    conn,
                )
                .await?,
                TournamentGameResult::new(&game_result),
            ),
            _ => unreachable!(),
        };
        let game = diesel::update(games::table.find(self.id))
            .set((
                finished.eq(true),
                tournament_game_result.eq(tgr.to_string()),
                game_status.eq(new_game_status.to_string()),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                white_rating.eq(w_rating),
                black_rating.eq(b_rating),
                white_rating_change.eq(w_change),
                black_rating_change.eq(b_change),
                updated_at.eq(Utc::now()),
                white_time_left.eq(white_time),
                black_time_left.eq(black_time),
                conclusion.eq(Conclusion::Resigned.to_string()),
            ))
            .get_result(conn)
            .await?;
        Ok(game)
    }

    pub async fn accept_draw(
        &self,
        game_control: &GameControl,
        conn: &mut DbConn<'_>,
    ) -> Result<Game, DbError> {
        let game_control_string = format!("{}. {game_control};", self.turn);
        let (white_time, black_time) = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left()?,
        };
        if white_time == Some(0) || black_time == Some(0) {
            return self.check_time(conn).await;
        }
        let tgr = TournamentGameResult::Draw;
        let (w_rating, b_rating, w_change, b_change) = Rating::update(
            self.rated,
            self.speed.clone(),
            self.white_id,
            self.black_id,
            GameResult::Draw,
            conn,
        )
        .await?;
        let game = diesel::update(games::table.find(self.id))
            .set((
                finished.eq(true),
                tournament_game_result.eq(tgr.to_string()),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                game_status.eq(GameStatus::Finished(GameResult::Draw).to_string()),
                white_rating.eq(w_rating),
                black_rating.eq(b_rating),
                white_rating_change.eq(w_change),
                black_rating_change.eq(b_change),
                updated_at.eq(Utc::now()),
                white_time_left.eq(white_time),
                black_time_left.eq(black_time),
                conclusion.eq(Conclusion::Draw.to_string()),
            ))
            .get_result(conn)
            .await?;
        Ok(game)
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

    // TODO: refactor into smaller functions
    pub async fn get_rows_from_options(
        options: &GamesQueryOptions,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        let mut query = users::table
            .inner_join(games_users::table.on(users::id.eq(games_users::user_id)))
            .inner_join(games::table.on(games_users::game_id.eq(games::id)))
            .into_boxed();

        match options.game_progress {
            GameProgress::Unstarted => {
                query = query.filter(
                    games::game_status
                        .eq("NotStarted")
                        .and(games::game_start.eq("Ready"))
                        .and(games::conclusion.ne("Committee")),
                );
            }
            GameProgress::Playing => {
                query = query.filter(
                    games::game_status.eq("InProgress").or(games::game_status
                        .eq("NotStarted")
                        .and(games::game_start.ne("Ready"))),
                );
            }
            GameProgress::Finished => {
                query = query.filter(games::finished.eq(true));
            }
            GameProgress::All => {}
        }

        //Selected speeds
        let speeds: Vec<_> = options.speeds.iter().map(|s| s.to_string()).collect();
        query = query.filter(games::speed.eq_any(speeds));

        //Multiple usernames. For each choose color and result type
        if !options.players.is_empty() {
            use users::normalized_username as username;
            let is_white = games::white_id.eq(users::id);
            let is_black = games::black_id.eq(users::id);
            let white_won = games::game_status
                .eq(GameStatus::Finished(GameResult::Winner(Color::White)).to_string());
            let black_won = games::game_status
                .eq(GameStatus::Finished(GameResult::Winner(Color::Black)).to_string());
            let is_draw = games::game_status.eq(GameStatus::Finished(GameResult::Draw).to_string());

            let mut query_conditions: Vec<Box<dyn BoxableExpression<_, _, SqlType = _>>> =
                Vec::new();

            for (usern, color, result) in options.players.iter() {
                let user_filter = username.eq(usern.to_lowercase());
                let result_filter: Option<Box<dyn BoxableExpression<_, _, SqlType = _>>> =
                    match result {
                        Some(ResultType::Win) => Some(match color {
                            Some(Color::White) => Box::new(white_won.clone().and(is_white)),
                            Some(Color::Black) => Box::new(black_won.clone().and(is_black)),
                            None => Box::new(
                                (white_won.clone().and(is_white))
                                    .or(black_won.clone().and(is_black)),
                            ),
                        }),
                        Some(ResultType::Loss) => Some(match color {
                            Some(Color::White) => Box::new(black_won.clone().and(is_white)),
                            Some(Color::Black) => Box::new(white_won.clone().and(is_black)),
                            None => Box::new(
                                (white_won.clone().and(is_black))
                                    .or(black_won.clone().and(is_white)),
                            ),
                        }),
                        Some(ResultType::Draw) => Some(match color {
                            Some(Color::White) => Box::new(is_white.and(is_draw.clone())),
                            Some(Color::Black) => Box::new(is_black.and(is_draw.clone())),
                            None => Box::new(
                                (is_white.and(is_draw.clone())).or(is_black.and(is_draw.clone())),
                            ),
                        }),
                        None => match color {
                            Some(Color::White) => Some(Box::new(is_white)),
                            Some(Color::Black) => Some(Box::new(is_black)),
                            None => None,
                        },
                    };

                let combined_filter: Box<dyn BoxableExpression<_, _, SqlType = _>> =
                    match result_filter {
                        Some(res) => Box::new(user_filter.and(res)),
                        None => Box::new(user_filter),
                    };

                query_conditions.push(combined_filter);
            }

            if !query_conditions.is_empty() {
                let combined_filter = query_conditions
                    .into_iter()
                    .reduce(|acc, cond| Box::new(acc.or(cond)));
                if let Some(filter) = combined_filter {
                    query = query.filter(filter);
                }
            }
        }

        query = query.order_by((games::updated_at.desc(), games::id.desc()));

        //batching
        if let Some(batch) = &options.current_batch {
            query = query.filter(diesel::BoolExpressionMethods::or(
                games::updated_at.lt(batch.timestamp),
                diesel::BoolExpressionMethods::and(
                    games::updated_at.eq(batch.timestamp),
                    games::id.ne(batch.id),
                ),
            ));
        }
        query = query.limit(options.batch_size as i64);
        Ok(query.select(games::all_columns).get_results(conn).await?)
    }

    pub async fn adjudicate_tournament_result(
        &self,
        user_id: &Uuid,
        new_result: &TournamentGameResult,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        match Conclusion::from_str(&self.conclusion) {
            Ok(Conclusion::Committee) | Ok(Conclusion::Unknown) | Ok(Conclusion::Forfeit) => {}
            _ => {
                return Err(DbError::InvalidAction {
                    info: String::from("You cannot adjudicate a played game"),
                });
            }
        }
        if let Some(tid) = self.tournament_id {
            let tournament = Tournament::find(tid, conn).await?;
            tournament
                .ensure_user_is_organizer_or_admin(user_id, conn)
                .await?;
            let con = match new_result {
                TournamentGameResult::DoubeForfeit => Conclusion::Forfeit,
                TournamentGameResult::Unknown => Conclusion::Unknown,
                _ => Conclusion::Committee,
            };
            let fin = new_result != &TournamentGameResult::Unknown;
            let game = diesel::update(games::table.find(self.id))
                .set((
                    finished.eq(fin),
                    conclusion.eq(con.to_string()),
                    game_status.eq(GameStatus::Adjudicated.to_string()),
                    tournament_game_result.eq(new_result.to_string()),
                    updated_at.eq(Utc::now()),
                    last_interaction.eq(Utc::now()),
                ))
                .get_result(conn)
                .await?;
            Ok(game)
        } else {
            Err(DbError::InvalidAction {
                info: String::from("Not a tournament game"),
            })
        }
    }

    pub async fn start(&self, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        if self.finished || self.turn > 0 || self.game_status != GameStatus::NotStarted.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot start this game"),
            });
        }
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_status.eq(GameStatus::InProgress.to_string()),
                updated_at.eq(Utc::now()),
                last_interaction.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    pub fn str_time_left_for_player(&self, player: Uuid) -> String {
        if let Some(color) = self.user_color(player) {
            if let Ok(time) = match color {
                Color::White => self.white_time_left_duration(),
                Color::Black => self.black_time_left_duration(),
            } {
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
}
