use crate::{
    db_error::DbError,
    models::user::User,
    schema::{
        challenges::{self, nanoid as nanoid_field, opponent_id as opponent_id_field},
        games::{self, nanoid as game_nanoid},
        users,
    },
    DbConn,
};
use chrono::prelude::*;
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::RunQueryDsl;
use nanoid::nanoid;
use serde::Serialize;
use shared_types::{ChallengeDetails, ChallengeId, TimeMode};
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
    pub async fn new(
        challenger_id: Uuid,
        opponent_id: Option<Uuid>,
        d: &ChallengeDetails,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        match d.time_mode {
            TimeMode::Untimed => {
                if d.time_base.is_some() || d.time_increment.is_some() {
                    return Err(DbError::InvalidInput {
                        info: String::from("Untimed game has time_base or time_increment"),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            d.time_base, d.time_increment
                        ),
                    });
                }
            }
            TimeMode::RealTime => {
                if d.time_base.is_none() || d.time_increment.is_none() {
                    return Err(DbError::InvalidInput {
                        info: String::from("Realtime game is missing time_base or time_increment"),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            d.time_base, d.time_increment
                        ),
                    });
                }
            }
            TimeMode::Correspondence => {
                if (d.time_base.is_some() && d.time_increment.is_some())
                    || (d.time_base.is_none() && d.time_increment.is_none())
                {
                    return Err(DbError::InvalidInput {
                        info: String::from(
                            "Correspondence game has wrong time_base or time_increment",
                        ),
                        error: format!(
                            "time_base: {:?}, time_increment: {:?}",
                            d.time_base, d.time_increment
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
        let mut nanoid: String;
        loop {
            nanoid = nanoid!(12);
            if let Ok(false) = select(exists(games::table.filter(game_nanoid.eq(&nanoid))))
                .get_result(conn)
                .await
            {
                break;
            }
        }
        Ok(Self {
            nanoid,
            challenger_id,
            opponent_id,
            game_type: d.game_type.to_string(),
            rated: d.rated,
            visibility: d.visibility.to_string(),
            tournament_queen_rule: true,
            color_choice: d.color_choice.to_string(),
            created_at: Utc::now(),
            time_mode: d.time_mode.to_string(),
            time_base: d.time_base,
            time_increment: d.time_increment,
            band_upper: d.band_upper,
            band_lower: d.band_lower,
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
    pub async fn create(
        new_challenge: &NewChallenge,
        conn: &mut DbConn<'_>,
    ) -> Result<Challenge, DbError> {
        Ok(diesel::insert_into(challenges::table)
            .values(new_challenge)
            .get_result(conn)
            .await?)
    }

    pub async fn get_public(conn: &mut DbConn<'_>) -> Result<Vec<Challenge>, DbError> {
        Ok(challenges::table
            .filter(challenges::visibility.eq("Public"))
            .get_results(conn)
            .await?)
    }

    pub async fn get_own(user: Uuid, conn: &mut DbConn<'_>) -> Result<Vec<Challenge>, DbError> {
        Ok(challenges::table
            .filter(challenges::challenger_id.eq(user))
            .get_results(conn)
            .await?)
    }

    pub async fn get_public_exclude_user(
        user: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Challenge>, DbError> {
        Ok(challenges::table
            .filter(challenges::visibility.eq("Public"))
            .filter(challenges::challenger_id.ne(user))
            .get_results(conn)
            .await?)
    }

    pub async fn direct_challenges(
        id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Challenge>, DbError> {
        Ok(challenges::table
            .filter(opponent_id_field.eq(id))
            .get_results(conn)
            .await?)
    }

    pub async fn find_by_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Challenge, DbError> {
        Ok(challenges::table.find(id).first(conn).await?)
    }

    pub async fn find_by_challenge_id(
        u: &ChallengeId,
        conn: &mut DbConn<'_>,
    ) -> Result<Challenge, DbError> {
        Ok(challenges::table
            .filter(nanoid_field.eq(u.0.clone()))
            .first(conn)
            .await?)
    }

    pub async fn get_challenger(&self, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users::table.find(&self.challenger_id).first(conn).await?)
    }

    pub async fn delete(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::delete(challenges::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
