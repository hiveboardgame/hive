use crate::{
    db_error::DbError,
    models::{Game, User},
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
    Clone,
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

#[derive(Clone, Copy)]
struct CounterDeltas {
    won: i64,
    lost: i64,
    drawn: i64,
}

impl CounterDeltas {
    const DRAW: Self = Self {
        won: 0,
        lost: 0,
        drawn: 1,
    };

    const WIN: Self = Self {
        won: 1,
        lost: 0,
        drawn: 0,
    };

    const LOSS: Self = Self {
        won: 0,
        lost: 1,
        drawn: 0,
    };
}

impl Rating {
    fn normalized_game_speed(game_speed: GameSpeed) -> String {
        match game_speed {
            GameSpeed::Untimed => GameSpeed::Correspondence.to_string(),
            _ => game_speed.to_string(),
        }
    }

    pub async fn for_uuid(
        uuid: &Uuid,
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let game_speed = Self::normalized_game_speed(*game_speed);
        Ok(ratings_table
            .filter(user_uid.eq(uuid).and(speed.eq(game_speed)))
            .first(conn)
            .await?)
    }

    pub async fn for_uuids_at_speed(
        uuids: &[Uuid],
        game_speed: &GameSpeed,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        let game_speed = Self::normalized_game_speed(*game_speed);
        Ok(ratings_table
            .filter(user_uid.eq_any(uuids).and(speed.eq(game_speed)))
            .load(conn)
            .await?)
    }

    pub async fn for_uuids(uuids: &[Uuid], conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        Ok(ratings_table
            .filter(user_uid.eq_any(uuids))
            .load(conn)
            .await?)
    }

    /// Prelocks every rating row a batch of game finalizations can touch.
    /// Individual finalizers reacquire their own rows harmlessly; the batch
    /// order prevents a multi-game transaction from deadlocking with another
    /// game after retaining a rating lock from an earlier finalization.
    pub(crate) async fn lock_for_game_updates(
        games: &[Game],
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let mut keys = Vec::with_capacity(games.len().saturating_mul(2));
        for game in games {
            let game_speed = GameSpeed::from_str(&game.speed).map_err(|error| {
                DbError::InvalidPersistedTournament {
                    reason: format!("game has invalid rating speed: {error}"),
                }
            })?;
            let rating_speed = Self::normalized_game_speed(game_speed);
            keys.push((rating_speed.clone(), game.white_id));
            keys.push((rating_speed, game.black_id));
        }
        keys.sort_unstable();
        keys.dedup();
        for (game_speed, player_id) in keys {
            Self::lock_for_update(player_id, &game_speed, conn).await?;
        }
        Ok(())
    }

    // Must be called inside the game-finalization transaction so these row locks
    // are held until the derived rating writes are complete.
    pub(crate) async fn update(
        rated: bool,
        game_speed: String,
        white_id: Uuid,
        black_id: Uuid,
        game_result: GameResult,
        effective_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<(f64, f64, Option<f64>, Option<f64>), DbError> {
        let game_speed =
            Self::normalized_game_speed(GameSpeed::from_str(&game_speed).map_err(|error| {
                DbError::InvalidPersistedTournament {
                    reason: format!("game has invalid rating speed: {error}"),
                }
            })?);
        if white_id == black_id {
            return Err(DbError::InvalidAction {
                info: "Cannot update ratings for self-play".to_string(),
            });
        }

        let first_id = white_id.min(black_id);
        let second_id = white_id.max(black_id);

        let first_rating = Self::lock_for_update(first_id, &game_speed, conn).await?;
        let second_rating = Self::lock_for_update(second_id, &game_speed, conn).await?;

        let (white_rating, black_rating) = if first_id == white_id {
            (first_rating, second_rating)
        } else {
            (second_rating, first_rating)
        };

        let (white_change, black_change) = Rating::apply_result(
            rated,
            game_result,
            &white_rating,
            &black_rating,
            effective_at,
            conn,
        )
        .await?;
        Ok((
            white_rating.rating,
            black_rating.rating,
            white_change,
            black_change,
        ))
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

    async fn apply_side(
        side: &Rating,
        glicko: Option<Glicko2Rating>,
        deltas: CounterDeltas,
        effective_at: DateTime<Utc>,
        conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    ) -> Result<(), DbError> {
        let counters = (
            played.eq(played + 1),
            won.eq(won + deltas.won),
            lost.eq(lost + deltas.lost),
            draw.eq(draw + deltas.drawn),
        );
        match glicko {
            Some(glicko) => {
                diesel::update(ratings::table.find(side.id))
                    .set((
                        counters,
                        updated_at.eq(effective_at),
                        rating.eq(glicko.rating),
                        deviation.eq(glicko.deviation),
                        volatility.eq(glicko.volatility),
                    ))
                    .execute(conn)
                    .await?;
            }
            None => {
                diesel::update(ratings::table.find(side.id))
                    .set(counters)
                    .execute(conn)
                    .await?;
            }
        }
        Ok(())
    }

    async fn apply_result(
        rated: bool,
        game_result: GameResult,
        white_rating: &Rating,
        black_rating: &Rating,
        effective_at: DateTime<Utc>,
        conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    ) -> Result<(Option<f64>, Option<f64>), DbError> {
        let (white_deltas, black_deltas) = match game_result {
            GameResult::Draw => (CounterDeltas::DRAW, CounterDeltas::DRAW),
            GameResult::Winner(Color::White) => (CounterDeltas::WIN, CounterDeltas::LOSS),
            GameResult::Winner(Color::Black) => (CounterDeltas::LOSS, CounterDeltas::WIN),
            GameResult::Unknown => unreachable!(
                "This function should not be called when there's no concrete game result"
            ),
        };

        let computed =
            rated.then(|| Rating::calculate_glicko2(white_rating, black_rating, game_result));
        let (white_glicko, black_glicko, white_change, black_change) = match computed {
            Some((white_glicko, black_glicko, white_change, black_change)) => (
                Some(white_glicko),
                Some(black_glicko),
                Some(white_change),
                Some(black_change),
            ),
            None => (None, None, None, None),
        };

        Self::apply_side(white_rating, white_glicko, white_deltas, effective_at, conn).await?;
        Self::apply_side(black_rating, black_glicko, black_deltas, effective_at, conn).await?;

        Ok((white_change, black_change))
    }
}
