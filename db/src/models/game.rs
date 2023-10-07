use crate::{
    get_conn,
    models::{game_user::GameUser, rating::Rating},
    schema::games,
    schema::games::dsl::*,
    DbPool,
};
use diesel::{prelude::*, result::Error, Identifiable, Insertable, QueryDsl, Queryable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use hive_lib::{
    color::Color, game_control::GameControl, game_result::GameResult, game_status::GameStatus,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::str::FromStr;

#[derive(Insertable, Debug)]
#[diesel(table_name = games)]
pub struct NewGame {
    pub nanoid: String,
    pub black_id: Uuid, // uid of user
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
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = games)]
pub struct Game {
    pub id: Uuid,
    pub nanoid: String,
    pub black_id: Uuid, // uid of user
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
}

impl Game {
    pub async fn create(new_game: &NewGame, pool: &DbPool) -> Result<Game, Error> {
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
        new_game_status: GameStatus,
        pool: &DbPool,
    ) -> Result<Game, Error> {
        let connection = &mut get_conn(pool).await?;
        if board_move.chars().last().unwrap_or(' ') != ';' {
            board_move = format!("{board_move};");
        }
        let mut game_control_string = String::new();
        if self.has_unanswered_game_control() {
            let gc = match self.last_game_control() {
                Some(GameControl::TakebackRequest(color)) => {
                    GameControl::TakebackReject(Color::from(color.opposite()))
                }
                Some(GameControl::DrawOffer(color)) => {
                    GameControl::DrawReject(Color::from(color.opposite()))
                }
                _ => unreachable!(),
            };
            game_control_string = format!("{}. {gc};", self.turn);
        }

        connection
            .transaction::<_, diesel::result::Error, _>(|conn| {
                async move {
                    let changes: Option<(f64, f64)> =
                        if let GameStatus::Finished(game_result) = new_game_status.clone() {
                            if let GameResult::Unknown = game_result {
                                None
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
                            None
                        };
                    let (w_change, b_change) = if let Some((white_change, black_change)) = changes {
                        (Some(white_change), Some(black_change))
                    } else {
                        (None, None)
                    };
                    let game = diesel::update(games::table.find(self.id))
                        .set((
                            history.eq(history.concat(board_move)),
                            turn.eq(turn + 1),
                            game_status.eq(new_game_status.to_string()),
                            game_control_history
                                .eq(game_control_history.concat(game_control_string)),
                            white_rating_change.eq(w_change),
                            black_rating_change.eq(b_change),
                        ))
                        .get_result(conn)
                        .await?;
                    Ok(game)
                }
                .scope_boxed()
            })
            .await
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
        game_control: GameControl,
        pool: &DbPool,
    ) -> Result<Game, Error> {
        let conn = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);
        diesel::update(games::table.find(self.id))
            .set(game_control_history.eq(game_control_history.concat(game_control_string)))
            .get_result(conn)
            .await
    }

    pub async fn accept_takeback(
        &self,
        new_history: String,
        new_game_status: String,
        game_control: GameControl,
        pool: &DbPool,
    ) -> Result<Game, Error> {
        let conn = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);

        diesel::update(games::table.find(self.id))
            .set((
                history.eq(new_history),
                turn.eq(turn - 1),
                game_status.eq(new_game_status),
                game_control_history.eq(game_control_history.concat(game_control_string)),
            ))
            .get_result(conn)
            .await
    }

    pub async fn resign(
        &self,
        game_control: GameControl,
        new_game_status: GameStatus,
        pool: &DbPool,
    ) -> Result<Game, Error> {
        let connection = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);

        connection
            .transaction::<_, diesel::result::Error, _>(|conn| {
                async move {
                    let changes: Option<(f64, f64)> = match new_game_status.clone() {
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
                            game_status.eq(new_game_status.to_string()),
                            game_control_history
                                .eq(game_control_history.concat(game_control_string)),
                            white_rating_change.eq(w_change),
                            black_rating_change.eq(b_change),
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
        game_control: GameControl,
        pool: &DbPool,
    ) -> Result<Game, Error> {
        let connection = &mut get_conn(pool).await?;
        let game_control_string = format!("{}. {game_control};", self.turn);
        connection
            .transaction::<_, diesel::result::Error, _>(|conn| {
                async move {
                    let changes: Option<(f64, f64)> = Rating::update(
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
                            game_control_history
                                .eq(game_control_history.concat(game_control_string)),
                            game_status.eq(GameStatus::Finished(GameResult::Draw).to_string()),
                            white_rating_change.eq(w_change),
                            black_rating_change.eq(b_change),
                        ))
                        .get_result(conn)
                        .await?;
                    Ok(game)
                }
                .scope_boxed()
            })
            .await
    }

    pub async fn set_status(&self, status: GameStatus, pool: &DbPool) -> Result<Game, Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::update(games::table.find(self.id))
            .set(game_status.eq(status.to_string()))
            .get_result(conn)
            .await
    }

    pub async fn find_by_uuid(uuid: &Uuid, pool: &DbPool) -> Result<Game, Error> {
        let conn = &mut get_conn(pool).await?;
        games::table.find(uuid).first(conn).await
    }

    pub async fn find_by_nanoid(find_nanoid: &str, pool: &DbPool) -> Result<Game, Error> {
        let conn = &mut get_conn(pool).await?;
        games::table.filter(nanoid.eq(find_nanoid)).first(conn).await
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(games::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
