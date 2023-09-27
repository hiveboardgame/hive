use crate::schema::{challenges, users};
use crate::{get_conn, DbPool};
use crate::models::user::User;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::result::Error;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = challenges)]
pub struct NewChallenge {
    pub challenger_uid: String,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Associations, Identifiable, Queryable, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(belongs_to(User, foreign_key = challenger_uid))]
#[diesel(table_name = challenges)]
pub struct Challenge {
    pub id: Uuid,
    pub challenger_uid: String,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    // TODO: periodically cleanup expired challanges
    pub created_at: DateTime<Utc>,
}

impl Challenge {
    pub async fn create(
        challenger: &User,
        challenge_request: &NewChallenge,
        pool: &DbPool,
    ) -> Result<Challenge, Error> {
        let conn = &mut get_conn(pool).await?;
        let new_challenge = NewChallenge {
            challenger_uid: challenger.uid.to_string(),
            color_choice: challenge_request.color_choice.to_string(),
            game_type: challenge_request.game_type.to_string(),
            rated: challenge_request.rated,
            public: challenge_request.public,
            tournament_queen_rule: challenge_request.tournament_queen_rule,
            created_at: Utc::now(),
        };
        diesel::insert_into(challenges::table)
            .values(&new_challenge)
            .get_result(conn)
            .await
    }

    pub async fn get_public(pool: &DbPool) -> Result<Vec<Challenge>, Error> {
        let conn = &mut get_conn(pool).await?;
        challenges::table
            .filter(challenges::public.eq(true))
            .get_results(conn)
            .await
    }

    pub async fn get(id: &Uuid, pool: &DbPool) -> Result<Challenge, Error> {
        let conn = &mut get_conn(pool).await?;
        challenges::table.find(id).first(conn).await
    }

    pub async fn get_challenger(&self, pool: &DbPool) -> Result<User, Error> {
        let conn = &mut get_conn(pool).await?;
        users::table.find(&self.challenger_uid).first(conn).await
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(challenges::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
