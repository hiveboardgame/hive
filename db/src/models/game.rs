use super::challenge::Challenge;
use crate::{
    db_error::DbError,
    get_conn,
    models::{game_user::GameUser, rating::Rating},
    schema::games,
    schema::games_users::dsl::games_users,
    schema::{challenges, challenges::nanoid as nanoid_field, games::dsl::*},
    DbPool,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Identifiable, Insertable, Queryable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use hive_lib::{
    color::Color, game_control::GameControl, game_result::GameResult, game_status::GameStatus,
    history::History, state::State,
};
use serde::{Deserialize, Serialize};
use shared_types::time_mode::TimeMode;
use shared_types::{conclusion::Conclusion, game_speed::GameSpeed};
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

static NANOS_IN_SECOND: u64 = 1000000000_u64;

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
}

impl NewGame {
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
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
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
}

impl Game {
    pub fn hashes(&self) -> Vec<u64> {
        // WARN: @leex reimplement this
        //self.hashes.iter().map(|i| *i as u64).collect::<Vec<u64>>()
        Vec::new()
    }

    pub async fn create(new_game: &NewGame, pool: &DbPool) -> Result<(Game, Vec<String>), DbError> {
        get_conn(pool)
            .await?
            .transaction::<_, DbError, _>(move |conn| {
                async move {
                    let game: Game = new_game.insert_into(games::table).get_result(conn).await?;
                    let game_user_white = GameUser::new(game.id, game.white_id);
                    game_user_white
                        .insert_into(games_users)
                        .execute(conn)
                        .await?;
                    let game_user_black = GameUser::new(game.id, game.black_id);
                    game_user_black
                        .insert_into(games_users)
                        .execute(conn)
                        .await?;
                    let challenge: Challenge = challenges::table
                        .filter(nanoid_field.eq(game.nanoid.clone()))
                        .first(conn)
                        .await?;
                    let mut deleted = Vec::new();
                    if let Ok(TimeMode::RealTime) = TimeMode::from_str(&challenge.time_mode) {
                        let challenges: Vec<Challenge> = challenges::table
                            .filter(
                                challenges::time_mode
                                    .eq(TimeMode::RealTime.to_string())
                                    .and(
                                        challenges::challenger_id
                                            .eq_any(&[game.white_id, game.black_id]),
                                    ),
                            )
                            .get_results(conn)
                            .await?;
                        for challenge in challenges {
                            deleted.push(challenge.nanoid.clone());
                            diesel::delete(challenges::table.find(challenge.id))
                                .execute(conn)
                                .await?;
                        }
                    } else {
                        deleted.push(challenge.nanoid.clone());
                        diesel::delete(challenges::table.find(challenge.id))
                            .execute(conn)
                            .await?;
                    };
                    Ok((game, deleted))
                }
                .scope_boxed()
            })
            .await
    }

    pub async fn check_time(&self, pool: &DbPool) -> Result<Game, DbError> {
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
            let new_game_status = GameStatus::Finished(game_result.clone());
            get_conn(pool)
                .await?
                .transaction::<_, DbError, _>(move |conn| {
                    async move {
                        let (w_rating, b_rating, w_change, b_change) = Rating::update(
                            self.rated,
                            &self.speed,
                            self.white_id,
                            self.black_id,
                            game_result,
                            conn,
                        )
                        .await?;
                        let game = diesel::update(games::table.find(self.id))
                            .set((
                                finished.eq(true),
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
                    }
                    .scope_boxed()
                })
                .await
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

    fn calculate_time_left_add_increment(&self) -> Result<(Option<i64>, Option<i64>), DbError> {
        let (mut white_time, mut black_time) = self.calculate_time_left()?;
        let increment = self.time_increment_duration()?.as_nanos() as i64;
        if self.turn % 2 == 0 {
            white_time = white_time.map(|time| time + increment);
        } else {
            black_time = black_time.map(|time| time + increment);
        };
        Ok((white_time, black_time))
    }

    pub async fn update_gamestate(
        &self,
        state: &State,
        pool: &DbPool,
    ) -> Result<Game, DbError> {
        let connection = &mut get_conn(pool).await?;
        let mut new_history = state.history.moves
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

        let mut interaction = None;
        let mut black_time = None;
        let mut white_time = None;
        let mut timed_out = false;
        let mut new_conclusion = Conclusion::Unknown;
        let mut new_game_status = state.game_status.clone();

        match new_game_status {
            GameStatus::Finished(GameResult::Draw) => new_conclusion = Conclusion::Board,
            GameStatus::Finished(GameResult::Winner(_)) => new_conclusion = Conclusion::Board,
            _ => {}
        }
        if state.repeating_moves.len() > 2 {
            new_conclusion = Conclusion::Repetition;
        }

        match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => {}
            TimeMode::RealTime => {
                if self.turn < 2 {
                    white_time = self.white_time_left;
                    black_time = self.black_time_left;
                } else {
                    (white_time, black_time) = self.calculate_time_left_add_increment()?;
                    if self.turn % 2 == 0 {
                        if white_time == Some(0) {
                            timed_out = true;
                            new_game_status =
                                GameStatus::Finished(GameResult::Winner(Color::Black));
                        }
                    } else if black_time == Some(0) {
                        timed_out = true;
                        new_game_status = GameStatus::Finished(GameResult::Winner(Color::White));
                    }
                }
                interaction = Some(Utc::now());
            }
            TimeMode::Correspondence => {
                if self.turn < 2 {
                    white_time = self.white_time_left;
                    black_time = self.black_time_left;
                } else {
                    (white_time, black_time) = self.calculate_time_left()?;
                    if self.turn % 2 == 0 {
                        if white_time == Some(0) {
                            timed_out = true;
                            new_game_status =
                                GameStatus::Finished(GameResult::Winner(Color::Black));
                        } else {
                            match (self.time_increment, self.time_base) {
                                (Some(inc), None) => {
                                    white_time = Some((inc as u64 * NANOS_IN_SECOND) as i64);
                                }
                                (None, Some(_)) => {}
                                _ => unreachable!(),
                            }
                        }
                    } else if black_time == Some(0) {
                        timed_out = true;
                        new_game_status = GameStatus::Finished(GameResult::Winner(Color::White));
                    } else {
                        match (self.time_increment, self.time_base) {
                            (Some(inc), None) => {
                                black_time = Some((inc as u64 * NANOS_IN_SECOND) as i64);
                            }
                            (None, Some(_)) => {}
                            _ => unreachable!(),
                        }
                    }
                }
                interaction = Some(Utc::now());
            }
        }
        let next_player = if state.turn % 2 == 0 {
            self.white_id
        } else {
            self.black_id
        };
        connection
            .transaction::<_, DbError, _>(move |conn| {
                async move {
                    if let GameStatus::Finished(game_result) = new_game_status.clone() {
                        if let GameResult::Unknown = game_result {
                            panic!("GameResult is unknown but the game is over");
                        };
                        let (w_rating, b_rating, w_change, b_change) = Rating::update(
                            self.rated,
                            &self.speed,
                            self.white_id,
                            self.black_id,
                            game_result,
                            conn,
                        )
                        .await?;
                        let new_turn = if timed_out { self.turn } else { state.turn as i32 };
                        if timed_out {
                            new_conclusion = Conclusion::Timeout;
                            new_history = self.history.clone();
                        }
                        let game = diesel::update(games::table.find(self.id))
                            .set((
                                history.eq(new_history),
                                current_player_id.eq(next_player),
                                turn.eq(new_turn),
                                finished.eq(true),
                                game_status.eq(new_game_status.to_string()),
                                game_control_history
                                    .eq(game_control_history.concat(game_control_string)),
                                white_rating.eq(w_rating),
                                black_rating.eq(b_rating),
                                white_rating_change.eq(w_change),
                                black_rating_change.eq(b_change),
                                updated_at.eq(Utc::now()),
                                white_time_left.eq(white_time),
                                black_time_left.eq(black_time),
                                last_interaction.eq(interaction),
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
                                game_status.eq(new_game_status.to_string()),
                                game_control_history
                                    .eq(game_control_history.concat(game_control_string)),
                                updated_at.eq(Utc::now()),
                                white_time_left.eq(white_time),
                                black_time_left.eq(black_time),
                                last_interaction.eq(interaction),
                            ))
                            .get_result(conn)
                            .await?;
                        Ok(game)
                    }
                }
                .scope_boxed()
            })
            .await
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
        if let Some(last) = self.game_control_history.split_terminator(';').last() {
            if let Some(gc) = last.split(' ').last() {
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
        pool: &DbPool,
    ) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    // TODO: get rid of new_game_status and compute it here
    pub async fn accept_takeback(
        &self,
        game_control: &GameControl,
        pool: &DbPool,
    ) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);

        let mut moves = self.history.split_terminator(';').collect::<Vec<_>>();
        let mut popped = 1;
        if let Some(a_move) = moves.pop() {
            if a_move.trim() == "pass" {
                moves.pop();
                popped += 1;
            }
        }
        let mut new_history = moves.join(";");
        if !new_history.is_empty() {
            new_history.push(';');
        };
        // TODO: now we have error problems here... get rid of the expects
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
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn resign(&self, game_control: &GameControl, pool: &DbPool) -> Result<Game, DbError> {
        let connection = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);

        let winner_color = game_control.color().opposite_color();
        let new_game_status = GameStatus::Finished(GameResult::Winner(winner_color));

        let (white_time, black_time) = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left()?,
        };
        if white_time == Some(0) || black_time == Some(0) {
            return self.check_time(pool).await;
        }
        connection
            .transaction::<_, DbError, _>(move |conn| {
                async move {
                    let (w_rating, b_rating, w_change, b_change) = match new_game_status.clone() {
                        GameStatus::Finished(game_result) => {
                            Rating::update(
                                self.rated,
                                &self.speed,
                                self.white_id,
                                self.black_id,
                                game_result.clone(),
                                conn,
                            )
                            .await?
                        }
                        _ => unreachable!(),
                    };
                    let game = diesel::update(games::table.find(self.id))
                        .set((
                            finished.eq(true),
                            game_status.eq(new_game_status.to_string()),
                            game_control_history
                                .eq(game_control_history.concat(game_control_string)),
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
                .scope_boxed()
            })
            .await
    }

    pub async fn accept_draw(
        &self,
        game_control: &GameControl,
        pool: &DbPool,
    ) -> Result<Game, DbError> {
        let connection = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);
        let (white_time, black_time) = match TimeMode::from_str(&self.time_mode)? {
            TimeMode::Untimed => (None, None),
            _ => self.calculate_time_left()?,
        };
        if white_time == Some(0) || black_time == Some(0) {
            return self.check_time(pool).await;
        }
        connection
            .transaction::<_, DbError, _>(move |conn| {
                async move {
                    let (w_rating, b_rating, w_change, b_change) = Rating::update(
                        self.rated,
                        &self.speed,
                        self.white_id,
                        self.black_id,
                        GameResult::Draw,
                        conn,
                    )
                    .await?;
                    let game = diesel::update(games::table.find(self.id))
                        .set((
                            finished.eq(true),
                            game_control_history
                                .eq(game_control_history.concat(game_control_string)),
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
                .scope_boxed()
            })
            .await
    }

    pub async fn set_status(&self, status: GameStatus, pool: &DbPool) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(diesel::update(games::table.find(self.id))
            .set((
                game_status.eq(status.to_string()),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn find_by_uuid(uuid: &Uuid, pool: &DbPool) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        let game: Game = games::table.find(uuid).first(conn).await?;
        if !game.finished && TimeMode::from_str(&game.time_mode)? != TimeMode::Untimed {
            game.check_time(pool).await
        } else {
            Ok(game)
        }
    }

    pub async fn find_by_nanoid(find_nanoid: &str, pool: &DbPool) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        let game: Game = games::table
            .filter(nanoid.eq(find_nanoid))
            .first(conn)
            .await?;
        if !game.finished && TimeMode::from_str(&game.time_mode)? != TimeMode::Untimed {
            game.check_time(pool).await
        } else {
            Ok(game)
        }
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), DbError> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(games::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
