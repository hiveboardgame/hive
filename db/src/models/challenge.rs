use crate::models::user::User;
use crate::schema::{challenges, users};
use crate::{get_conn, DbPool};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::result::Error;
use diesel_async::RunQueryDsl;
use hive_lib::game_type::GameType;
use nanoid::nanoid;
use serde::Serialize;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = challenges)]
pub struct NewChallenge {
    pub url: String,
    pub challenger_id: Uuid,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    pub created_at: DateTime<Utc>,
}

impl NewChallenge {
    pub fn new(
        challenger_id: Uuid,
        game_type: GameType,
        rated: bool,
        public: bool,
        tournament_queen_rule: bool,
        color_choice: String,
    ) -> Self {
        Self {
            url: nanoid!(10),
            challenger_id,
            game_type: game_type.to_string(),
            rated,
            public,
            tournament_queen_rule,
            color_choice,
            created_at: Utc::now(),
        }
    }
}

#[derive(Associations, Identifiable, Queryable, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(belongs_to(User, foreign_key = challenger_id))]
#[diesel(table_name = challenges)]
pub struct Challenge {
    pub id: Uuid,
    pub url: String,
    pub challenger_id: Uuid,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    // TODO: periodically cleanup expired challanges
    pub created_at: DateTime<Utc>,
}

impl Challenge {
    pub async fn create(new_challenge: &NewChallenge, pool: &DbPool) -> Result<Challenge, Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::insert_into(challenges::table)
            .values(new_challenge)
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
        users::table.find(&self.challenger_id).first(conn).await
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<(), Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(challenges::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }
}
