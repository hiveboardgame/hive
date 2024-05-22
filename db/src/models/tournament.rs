use crate::{
    db_error::DbError,
    get_conn,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
    },
    schema::{
        tournaments::{self, invitees as invitees_column, series as series_column},
        users,
    },
    DbPool,
};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::time_mode::TimeMode;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournaments)]
pub struct NewTournament {
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub invitees: Vec<Option<Uuid>>,
    pub seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl NewTournament {
    pub fn new(
        series: Option<Uuid>,
        name: String,
        description: String,
        scoring: String,
        tiebreaker: Vec<Option<String>>,
        invitees: Vec<Option<Uuid>>,
        seats: i32,
        rounds: i32,
        joinable: bool,
        invite_only: bool,
        mode: String,
        time_mode: String,
        time_base: Option<i32>,
        time_increment: Option<i32>,
        band_upper: Option<i32>,
        band_lower: Option<i32>,
    ) -> Self {
        if matches!(TimeMode::from_str(&time_mode).unwrap(), TimeMode::Untimed) {
            panic!("You cannot play untimed tournaments");
        }

        Self {
            nanoid: nanoid!(11),
            name,
            description,
            scoring,
            tiebreaker,
            invitees,
            seats,
            rounds,
            joinable,
            invite_only,
            mode,
            time_mode,
            time_base,
            time_increment,
            band_upper,
            band_lower,
            start_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series,
        }
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournaments)]
pub struct Tournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub invitees: Vec<Option<Uuid>>,
    pub seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl Tournament {
    pub async fn create(
        new_tournament: &NewTournament,
        pool: &DbPool,
    ) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(diesel::insert_into(tournaments::table)
            .values(new_tournament)
            .get_result(connection)
            .await?)
    }

    pub async fn decline_invitation(
        &mut self,
        id: &Uuid,
        pool: &DbPool,
    ) -> Result<Tournament, DbError> {
        let conn = &mut get_conn(pool).await?;
        let mut still_invited = self.invitees.clone();
        still_invited.retain(|invited| *invited != Some(*id));
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(invitees_column.eq(still_invited))
            .get_result(conn)
            .await?)
    }

    pub async fn accept_invitation(
        &mut self,
        user_id: &Uuid,
        pool: &DbPool,
    ) -> Result<Tournament, DbError> {
        let mut still_invited = self.invitees.clone();
        still_invited.retain(|invited| *invited != Some(*user_id));

        get_conn(pool)
            .await?
            .transaction::<_, DbError, _>(move |conn| {
                async move {
                    let tournament_user = TournamentUser::new(self.id, *user_id);
                    tournament_user.insert(pool).await?;
                    Ok(diesel::update(tournaments::table.find(self.id))
                        .set(invitees_column.eq(still_invited))
                        .get_result(conn)
                        .await?)
                }
                .scope_boxed()
            })
            .await
    }

    pub async fn add_to_series(
        &self,
        series_id: Uuid,
        pool: &DbPool,
    ) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(Some(series_id)))
            .get_result(connection)
            .await?)
    }

    pub async fn remove_from_series(&self, pool: &DbPool) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(None::<Uuid>))
            .get_result(connection)
            .await?)
    }

    pub async fn join(&self, user_id: &Uuid, pool: &DbPool) -> Result<(), DbError> {
        let tournament_user = TournamentUser::new(self.id, *user_id);
        tournament_user.insert(pool).await?;
        Ok(())
    }

    pub async fn leave(&self, user_id: &Uuid, pool: &DbPool) -> Result<(), DbError> {
        TournamentUser::delete(self.id, *user_id, pool).await?;
        Ok(())
    }

    pub async fn from_uuid(uuid: &Uuid, pool: &DbPool) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(tournaments::table.find(uuid).first(connection).await?)
    }

    pub async fn players(&self, pool: &DbPool) -> Result<Vec<User>, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(connection)
            .await?)
    }

    pub async fn organizers(&self, pool: &DbPool) -> Result<Vec<User>, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(TournamentOrganizer::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(connection)
            .await?)
    }
}
