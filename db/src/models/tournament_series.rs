use crate::{
    db_error::DbError,
    schema::{tournament_series, users},
    DbConn,
};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{tournament_series_organizer::TournamentSeriesOrganizer, user::User};

#[derive(Insertable, Debug)]
#[diesel(table_name = tournament_series)]
pub struct NewTournamentSeries {
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NewTournamentSeries {
    pub fn new(name: String, description: String) -> Self {
        Self {
            nanoid: nanoid!(10),
            name,
            description,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournament_series)]
pub struct TournamentSeries {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TournamentSeries {
    pub async fn create(
        new_tournament_series: &NewTournamentSeries,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentSeries, DbError> {
        Ok(diesel::insert_into(tournament_series::table)
            .values(new_tournament_series)
            .get_result(conn)
            .await?)
    }

    pub async fn organizers(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentSeriesOrganizer::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }
}
