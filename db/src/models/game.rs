use super::challenge::Challenge;
use crate::{
    db_error::DbError,
    get_conn,
    models::{game_user::GameUser, rating::Rating},
    schema::games,
    schema::games::dsl::*,
    DbPool,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Identifiable, Insertable, QueryDsl, Queryable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use hive_lib::{
    color::Color, game_control::GameControl, game_result::GameResult, game_status::GameStatus,
    history::History, state::State,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

static NANOS_IN_A_DAY: i64 = 86400000000000_i64;
static NANOS_IN_MINUTE: i64 = 60000000000_i64;

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
    pub time_base: Option<i32>,      // Secons
    pub time_increment: Option<i32>, // Seconds
    pub last_interaction: Option<DateTime<Utc>>, // When was the last move made
    pub black_time_left: Option<i64>,
    pub white_time_left: Option<i64>,
}

impl NewGame {
    pub fn new(white: Uuid, black: Uuid, challenge: &Challenge) -> Self {
        let time_left = match challenge.time_mode.as_ref() {
            "Unlimited" => None,
            "Real Time" => Some(challenge.time_base.unwrap() as i64 * NANOS_IN_MINUTE),
            "Correspondence" => Some(challenge.time_base.unwrap() as i64 * NANOS_IN_MINUTE),
            _ => unimplemented!(),
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
    pub time_base: Option<i32>,      // Secons
    pub time_increment: Option<i32>, // Seconds
    pub last_interaction: Option<DateTime<Utc>>, // When was the last move made
    pub black_time_left: Option<i64>,
    pub white_time_left: Option<i64>,
}

impl Game {
    pub async fn create(new_game: &NewGame, pool: &DbPool) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        let game: Game = new_game.insert_into(games::table).get_result(conn).await?;
        let game_user_white = GameUser::new(game.id, game.white_id);
        game_user_white.insert(pool).await?;
        let game_user_black = GameUser::new(game.id, game.black_id);
        game_user_black.insert(pool).await?;
        Ok(game)
    }

    pub async fn make_move(
        &self,
        mut board_move: String,
        mut new_game_status: GameStatus,
        pool: &DbPool,
    ) -> Result<Game, DbError> {
        let connection = &mut get_conn(pool).await?;
        if board_move.chars().last().unwrap_or(' ') != ';' {
            board_move = format!("{board_move};");
        }
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

        if self.time_mode == "Real Time" {
            if self.turn == 0 {
                white_time = Some(self.time_base.unwrap() as i64 * NANOS_IN_MINUTE);
                black_time = Some(self.time_base.unwrap() as i64 * NANOS_IN_MINUTE);
            } else {
                // get time left for the current player
                let mut time_left = if self.turn % 2 == 0 {
                    Duration::from_nanos(self.white_time_left.unwrap() as u64)
                } else {
                    Duration::from_nanos(self.black_time_left.unwrap() as u64)
                };

                // if the player has time left
                if let Some(last) = self.last_interaction {
                    let time_passed = Utc::now().signed_duration_since(last).to_std().unwrap();
                    println!("Time left: {:?}", time_left);
                    println!("Time passed: {:?}", time_passed);
                    if time_left > time_passed {
                        // substract passed time and add time_increment
                        time_left = time_left - time_passed
                            + Duration::from_secs(self.time_increment.unwrap() as u64);
                        if self.turn % 2 == 0 {
                            black_time = self.black_time_left;
                            white_time = Some(time_left.as_nanos() as i64);
                        } else {
                            white_time = self.white_time_left;
                            black_time = Some(time_left.as_nanos() as i64);
                        };
                    } else {
                        if self.turn % 2 == 0 {
                            white_time = Some(0 as i64);
                            black_time = self.black_time_left;
                            new_game_status =
                                GameStatus::Finished(GameResult::Winner(Color::Black));
                        } else {
                            black_time = Some(0);
                            white_time = self.white_time_left;
                            new_game_status =
                                GameStatus::Finished(GameResult::Winner(Color::White));
                        }
                    }
                }
            }
            interaction = Some(Utc::now());
        }
        if self.time_mode == "Correspondence" {
            // get time left for the current player
            let time_left = if self.turn % 2 == 0 {
                Duration::from_nanos(self.white_time_left.unwrap() as u64)
            } else {
                Duration::from_nanos(self.black_time_left.unwrap() as u64)
            };

            // if the player has time left
            if let Some(last) = self.last_interaction {
                let time_passed = last.signed_duration_since(Utc::now()).to_std().unwrap();
                if time_left > time_passed {
                    // reset the time to X days
                    if self.turn % 2 == 0 {
                        black_time = self.black_time_left;
                        white_time = Some(self.time_base.unwrap() as i64 * NANOS_IN_A_DAY);
                    } else {
                        white_time = self.white_time_left;
                        black_time = Some(self.time_base.unwrap() as i64 * NANOS_IN_A_DAY);
                    };
                }
            }
            interaction = Some(Utc::now());
        }

        connection
            .transaction::<_, DbError, _>(move |conn| {
                let next_player = if self.current_player_id == self.black_id {
                    self.white_id
                } else {
                    self.black_id
                };
                async move {
                    let ((w_rating, b_rating), changes) =
                        if let GameStatus::Finished(game_result) = new_game_status.clone() {
                            if let GameResult::Unknown = game_result {
                                ((0.0, 0.0), None)
                            } else {
                                Rating::update(
                                    self.rated,
                                    self.white_id,
                                    self.black_id,
                                    game_result,
                                    conn,
                                )
                                .await?
                            }
                        } else {
                            ((0.0, 0.0), None)
                        };
                    let game = if let Some((w_change, b_change)) = changes {
                        diesel::update(games::table.find(self.id))
                            .set((
                                history.eq(history.concat(board_move)),
                                current_player_id.eq(next_player),
                                turn.eq(turn + 1),
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
                            ))
                            .get_result(conn)
                            .await?
                    } else {
                        diesel::update(games::table.find(self.id))
                            .set((
                                history.eq(history.concat(board_move)),
                                current_player_id.eq(next_player),
                                turn.eq(turn + 1),
                                game_status.eq(new_game_status.to_string()),
                                game_control_history
                                    .eq(game_control_history.concat(game_control_string)),
                                updated_at.eq(Utc::now()),
                                white_time_left.eq(white_time),
                                black_time_left.eq(black_time),
                                last_interaction.eq(interaction),
                            ))
                            .get_result(conn)
                            .await?
                    };
                    Ok(game)
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
        if let Some(gc) = self.last_game_control() {
            return matches!(
                gc,
                GameControl::TakebackRequest(_) | GameControl::DrawOffer(_)
            );
        }
        false
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
        if let Some(a_move) = moves.pop() {
            if a_move.trim() == "pass" {
                println!("found a pass, will delete another move");
                moves.pop();
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
                turn.eq(turn - 1),
                game_status.eq(new_game_status),
                game_control_history.eq(game_control_history.concat(game_control_string)),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn resign(&self, game_control: &GameControl, pool: &DbPool) -> Result<Game, DbError> {
        let connection = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);

        let winner_color = game_control.color().opposite_color();
        let new_game_status = GameStatus::Finished(GameResult::Winner(winner_color));

        connection
            .transaction::<_, DbError, _>(|conn| {
                async move {
                    let ((w_rating, b_rating), changes) = match new_game_status.clone() {
                        GameStatus::Finished(game_result) => {
                            Rating::update(
                                self.rated,
                                self.white_id,
                                self.black_id,
                                game_result.clone(),
                                conn,
                            )
                            .await?
                        }
                        _ => unreachable!(),
                    };

                    let (w_change, b_change) = if let Some((white_change, black_change)) = changes {
                        (Some(white_change), Some(black_change))
                    } else {
                        (None, None)
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
        connection
            .transaction::<_, DbError, _>(|conn| {
                async move {
                    let ((w_rating, b_rating), changes) = Rating::update(
                        self.rated,
                        self.white_id,
                        self.black_id,
                        GameResult::Draw,
                        conn,
                    )
                    .await?;
                    let (w_change, b_change) = if let Some((white_change, black_change)) = changes {
                        (Some(white_change), Some(black_change))
                    } else {
                        (None, None)
                    };
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
        Ok(games::table.find(uuid).first(conn).await?)
    }

    pub async fn find_by_nanoid(find_nanoid: &str, pool: &DbPool) -> Result<Game, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(games::table
            .filter(nanoid.eq(find_nanoid))
            .first(conn)
            .await?)
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), DbError> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(games::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
