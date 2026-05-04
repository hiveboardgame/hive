use crate::{
    db_error::DbError,
    models::User,
    schema::ratings::{self, dsl::ratings as ratings_table, *},
    DbConn,
};
use bb8::PooledConnection;
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    AsChangeset,
    Associations,
    Identifiable,
    Insertable,
    Queryable,
    Selectable,
};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager,
    AsyncPgConnection,
    RunQueryDsl,
};
use hive_lib::{Color, GameResult};
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;
use skillratings::{
    glicko2::{glicko2, Glicko2Config, Glicko2Rating},
    Outcomes,
};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = ratings)]
pub struct NewRating {
    pub user_uid: Uuid,
    pub played: i64,
    pub won: i64,
    pub lost: i64,
    pub draw: i64,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub speed: String,
}

impl NewRating {
    pub fn for_uuid(uuid: &Uuid, game_speed: GameSpeed) -> Self {
        Self {
            user_uid: uuid.to_owned(),
            played: 0,
            won: 0,
            lost: 0,
            draw: 0,
            rating: 1500.0,
            deviation: 500.0,
            volatility: 0.09,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            speed: game_speed.to_string(),
        }
    }
}

#[derive(
    Associations,
    Identifiable,
    Queryable,
    Debug,
    Serialize,
    Deserialize,
    AsChangeset,
    Selectable,
    PartialEq,
)]
#[serde(rename_all = "camelCase")]
#[diesel(belongs_to(User, foreign_key = user_uid))]
#[diesel(table_name = ratings)]
#[diesel(primary_key(id))]
pub struct Rating {
    pub id: i32,
    pub user_uid: Uuid,
    pub played: i64,
    pub won: i64,
    pub lost: i64,
    pub draw: i64,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub speed: String,
}

impl Rating {
    fn normalize_game_speed(game_speed: String) -> String {
        if GameSpeed::from_str(&game_speed).expect("Valid GameSpeed") == GameSpeed::Untimed {
            GameSpeed::Correspondence.to_string()
        } else {
            game_speed
        }
    }

    pub async fn for_uuid(
        uuid: &Uuid,
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let game_speed = match game_speed {
            GameSpeed::Untimed => GameSpeed::Correspondence.to_string(),
            _ => game_speed.to_string(),
        };
        Ok(ratings_table
            .filter(user_uid.eq(uuid).and(speed.eq(game_speed)))
            .first(conn)
            .await?)
    }

    // Must be called inside the game-finalization transaction so these row locks
    // are held until the derived rating writes are complete.
    pub(crate) async fn update(
        rated: bool,
        game_speed: String,
        white_id: Uuid,
        black_id: Uuid,
        game_result: GameResult,
        conn: &mut DbConn<'_>,
    ) -> Result<(f64, f64, Option<f64>, Option<f64>), DbError> {
        let game_speed = Self::normalize_game_speed(game_speed);
        let (white_rating, black_rating) =
            Self::lock_pair_for_update(&game_speed, white_id, black_id, conn).await?;

        let (white_change, black_change) = match game_result {
            GameResult::Draw => Rating::draw(rated, &white_rating, &black_rating, conn).await,
            GameResult::Winner(color) => {
                Rating::winner(rated, color, &white_rating, &black_rating, conn).await
            }
            GameResult::Unknown => unreachable!(
                "This function should not be called when there's no concrete game result"
            ),
        }?;
        Ok((
            white_rating.rating,
            black_rating.rating,
            white_change,
            black_change,
        ))
    }

    async fn lock_pair_for_update(
        game_speed: &str,
        white_id: Uuid,
        black_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(Rating, Rating), DbError> {
        debug_assert_ne!(white_id, black_id, "Cannot update ratings for self-play");

        let (first_id, second_id) = if white_id < black_id {
            (white_id, black_id)
        } else {
            (black_id, white_id)
        };

        let first_rating = Self::lock_for_update(first_id, game_speed, conn).await?;
        let second_rating = Self::lock_for_update(second_id, game_speed, conn).await?;

        if first_rating.user_uid == white_id {
            Ok((first_rating, second_rating))
        } else {
            Ok((second_rating, first_rating))
        }
    }

    async fn lock_for_update(
        player_id: Uuid,
        game_speed: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Rating, DbError> {
        Ok(ratings_table
            .filter(user_uid.eq(player_id))
            .filter(speed.eq(game_speed))
            .for_update()
            .first(conn)
            .await?)
    }

    fn calculate_glicko2(
        white_rating: &Rating,
        black_rating: &Rating,
        game_result: GameResult,
    ) -> (Glicko2Rating, Glicko2Rating, f64, f64) {
        let white_glicko = Glicko2Rating {
            rating: white_rating.rating,
            deviation: white_rating.deviation,
            volatility: white_rating.volatility,
        };

        let black_glicko = Glicko2Rating {
            rating: black_rating.rating,
            deviation: black_rating.deviation,
            volatility: black_rating.volatility,
        };

        let config = Glicko2Config {
            tau: 0.75,
            ..Default::default()
        };
        let outcome = match game_result {
            GameResult::Winner(winner) => {
                if winner == Color::White {
                    Outcomes::WIN
                } else {
                    Outcomes::LOSS
                }
            }
            GameResult::Draw => Outcomes::DRAW,
            GameResult::Unknown => unreachable!(),
        };
        let (white_glicko_new, black_glicko_new) =
            glicko2(&white_glicko, &black_glicko, &outcome, &config);
        (
            white_glicko_new,
            black_glicko_new,
            white_glicko_new.rating - white_glicko.rating,
            black_glicko_new.rating - black_glicko.rating,
        )
    }

    async fn draw(
        rated: bool,
        white_rating: &Rating,
        black_rating: &Rating,
        conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    ) -> Result<(Option<f64>, Option<f64>), DbError> {
        if rated {
            let (white_glicko, black_glicko, white_rating_change, black_rating_change) =
                Rating::calculate_glicko2(white_rating, black_rating, GameResult::Draw);
            diesel::update(ratings::table.find(black_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    draw.eq(draw + 1),
                    rating.eq(black_glicko.rating),
                    deviation.eq(black_glicko.deviation),
                    volatility.eq(black_glicko.volatility),
                ))
                .execute(conn)
                .await?;

            diesel::update(ratings::table.find(white_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    draw.eq(draw + 1),
                    rating.eq(white_glicko.rating),
                    deviation.eq(white_glicko.deviation),
                    volatility.eq(white_glicko.volatility),
                ))
                .execute(conn)
                .await?;
            Ok((Some(white_rating_change), Some(black_rating_change)))
        } else {
            diesel::update(ratings::table.find(black_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    draw.eq(draw + 1),
                ))
                .execute(conn)
                .await?;

            diesel::update(ratings::table.find(white_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    draw.eq(draw + 1),
                ))
                .execute(conn)
                .await?;
            Ok((None, None))
        }
    }

    async fn winner(
        rated: bool,
        winner: Color,
        white_rating: &Rating,
        black_rating: &Rating,
        conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    ) -> Result<(Option<f64>, Option<f64>), DbError> {
        let (white_won, white_lost) = {
            if winner == Color::White {
                (1, 0)
            } else {
                (0, 1)
            }
        };

        if rated {
            let (white_glicko, black_glicko, white_rating_change, black_rating_change) =
                Rating::calculate_glicko2(white_rating, black_rating, GameResult::Winner(winner));

            diesel::update(ratings::table.find(white_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    won.eq(won + white_won),
                    lost.eq(lost + white_lost),
                    rating.eq(white_glicko.rating),
                    deviation.eq(white_glicko.deviation),
                    volatility.eq(white_glicko.volatility),
                ))
                .execute(conn)
                .await?;

            diesel::update(ratings::table.find(black_rating.id))
                .set((
                    updated_at.eq(Utc::now()),
                    played.eq(played + 1),
                    won.eq(won + white_lost),
                    lost.eq(lost + white_won),
                    rating.eq(black_glicko.rating),
                    deviation.eq(black_glicko.deviation),
                    volatility.eq(black_glicko.volatility),
                ))
                .execute(conn)
                .await?;
            Ok((Some(white_rating_change), Some(black_rating_change)))
        } else {
            diesel::update(ratings::table.find(white_rating.id))
                .set((
                    played.eq(played + 1),
                    won.eq(won + white_won),
                    lost.eq(lost + white_lost),
                ))
                .execute(conn)
                .await?;

            diesel::update(ratings::table.find(black_rating.id))
                .set((
                    played.eq(played + 1),
                    won.eq(won + white_lost),
                    lost.eq(lost + white_won),
                ))
                .execute(conn)
                .await?;
            Ok((None, None))
        }
    }
}
