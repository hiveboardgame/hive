use crate::{
    db_error::DbError,
    get_conn,
    models::user::User,
    schema::{
        challenges, challenges::nanoid as nanoid_field,
        challenges::opponent_id as opponent_id_field, users,
    },
    DbPool,
};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::game_type::GameType;
use nanoid::nanoid;
use serde::Serialize;
use shared_types::TimeMode;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = challenges)]
pub struct NewChallenge {
    pub nanoid: String,
    pub challenger_id: Uuid,
    pub game_type: String,
    pub rated: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    pub created_at: DateTime<Utc>,
    pub opponent_id: Option<Uuid>,
    pub visibility: String,
    pub time_mode: String,           // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Seconds
    pub time_increment: Option<i32>, // Seconds
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}

impl NewChallenge {
    pub fn new(
        challenger_id: Uuid,
        opponent_id: Option<Uuid>,
        game_type: GameType,
        rated: bool,
        visibility: String,
        color_choice: String,
        time_mode: TimeMode,         // Correspondence, Timed, Untimed
        time_base: Option<i32>,      // Secons
        time_increment: Option<i32>, // Seconds
        band_upper: Option<i32>,
        band_lower: Option<i32>,
    ) -> Result<Self, DbError> {
        match time_mode {
            TimeMode::Untimed => {
                if time_base.is_some() || time_increment.is_some() {
                    return Err(DbError::InvalidInput {
                        info: String::from("Untimed game has time_base or time_increment"),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            time_base, time_increment
                        ),
                    });
                }
            }
            TimeMode::RealTime => {
                if time_base.is_none() || time_increment.is_none() {
                    return Err(DbError::InvalidInput {
                        info: String::from("Realtime game is missing time_base or time_increment"),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            time_base, time_increment
                        ),
                    });
                }
            }
            TimeMode::Correspondence => {
                if (time_base.is_some() && time_increment.is_some())
                    || (time_base.is_none() && time_increment.is_none())
                {
                    return Err(DbError::InvalidInput {
                        info: String::from(
                            "Correspondence game has wrong time_base or time_increment",
                        ),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            time_base, time_increment
                        ),
                    });
                }
            }
        }
        if opponent_id == Some(challenger_id) {
            return Err(DbError::InvalidInput {
                info: "You can't play here with yourself.".to_string(),
                error: String::new(),
            });
        };
        Ok(Self {
            nanoid: nanoid!(10),
            challenger_id,
            opponent_id,
            game_type: game_type.to_string(),
            rated,
            visibility,
            tournament_queen_rule: true,
            color_choice,
            created_at: Utc::now(),
            time_mode: time_mode.to_string(),
            time_base,
            time_increment,
            band_upper,
            band_lower,
        })
    }
}

#[derive(Associations, Identifiable, Queryable, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(belongs_to(User, foreign_key = challenger_id))]
#[diesel(table_name = challenges)]
pub struct Challenge {
    pub id: Uuid,
    pub nanoid: String,
    pub challenger_id: Uuid,
    pub game_type: String,
    pub rated: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    // TODO: periodically cleanup expired challanges
    pub created_at: DateTime<Utc>,
    pub opponent_id: Option<Uuid>,
    pub visibility: String,
    pub time_mode: String,           // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Secons
    pub time_increment: Option<i32>, // Seconds
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}

impl Challenge {
    pub async fn create(new_challenge: &NewChallenge, pool: &DbPool) -> Result<Challenge, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(diesel::insert_into(challenges::table)
            .values(new_challenge)
            .get_result(conn)
            .await?)
    }

    pub async fn get_public(pool: &DbPool) -> Result<Vec<Challenge>, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table
            .filter(challenges::visibility.eq("Public"))
            .get_results(conn)
            .await?)
    }

    pub async fn get_own(user: Uuid, pool: &DbPool) -> Result<Vec<Challenge>, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table
            .filter(challenges::challenger_id.eq(user))
            .get_results(conn)
            .await?)
    }

    pub async fn get_public_exclude_user(
        user: Uuid,
        pool: &DbPool,
    ) -> Result<Vec<Challenge>, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table
            .filter(challenges::visibility.eq("Public"))
            .filter(challenges::challenger_id.ne(user))
            .get_results(conn)
            .await?)
    }

    pub async fn direct_challenges(id: Uuid, pool: &DbPool) -> Result<Vec<Challenge>, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table
            .filter(opponent_id_field.eq(id))
            .get_results(conn)
            .await?)
    }

    pub async fn find_by_uuid(id: &Uuid, pool: &DbPool) -> Result<Challenge, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table.find(id).first(conn).await?)
    }

    pub async fn find_by_nanoid(u: &str, pool: &DbPool) -> Result<Challenge, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(challenges::table
            .filter(nanoid_field.eq(u))
            .first(conn)
            .await?)
    }

    pub async fn get_challenger(&self, pool: &DbPool) -> Result<User, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(users::table.find(&self.challenger_id).first(conn).await?)
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), DbError> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(challenges::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
