use crate::{
    db_error::DbError,
    get_conn,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
    },
    schema::{
        tournaments::nanoid as nanoid_field,
        tournaments::{self, invitees as invitees_column, series as series_column},
        tournaments_organizers, users,
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
use shared_types::{TimeMode, TournamentDetails};
use uuid::Uuid;

use super::Game;

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
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl NewTournament {
    pub fn new(details: TournamentDetails) -> Result<Self, DbError> {
        if matches!(details.time_mode, TimeMode::Untimed) {
            return Err(DbError::InvalidInput {
                info: String::from("How did you trigger this?"),
                error: String::from("Cannot create untimed tournament."),
            });
        }

        // TOOD: @leex add some validations

        Ok(Self {
            nanoid: nanoid!(11),
            name: details.name,
            description: details.description,
            scoring: details.scoring,
            tiebreaker: details.tiebreaker,
            invitees: details.invitees,
            seats: details.seats,
            rounds: details.rounds,
            joinable: details.joinable,
            invite_only: details.invite_only,
            mode: details.mode,
            time_mode: details.time_mode.to_string(),
            time_base: details.time_base,
            time_increment: details.time_increment,
            band_upper: details.band_upper,
            band_lower: details.band_lower,
            start_at: details.start_at,
            status: String::from("NotStarted"), // TODO @leex make this an enum
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: details.series,
        })
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
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl Tournament {
    pub async fn create(
        user_id: Uuid,
        new_tournament: &NewTournament,
        pool: &DbPool,
    ) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        // TODO: create only works when user's rating is RANKABLE
        connection
            .transaction::<_, DbError, _>(|conn| {
                async move {
                    let tournament: Tournament = diesel::insert_into(tournaments::table)
                        .values(new_tournament)
                        .get_result(conn)
                        .await?;
                    let tournament_organizer = TournamentOrganizer::new(tournament.id, user_id);
                    diesel::insert_into(tournaments_organizers::table)
                        .values(tournament_organizer)
                        .execute(conn)
                        .await?;
                    Ok(tournament)
                }
                .scope_boxed()
            })
            .await
    }

    pub async fn delete(&mut self, user_id: Uuid, pool: &DbPool) -> Result<(), DbError> {
        let organizers = self.organizers(pool).await?;
        if organizers.iter().any(|o| o.id == user_id) {
            let connection = &mut get_conn(pool).await?;
            diesel::delete(tournaments::table.find(self.id))
                .execute(connection)
                .await?;
        }
        Ok(())
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

    pub async fn join(&self, user_id: &Uuid, pool: &DbPool) -> Result<Tournament, DbError> {
        let players = self.players(pool).await?;
        if players.len() as i32 == self.seats {
            return Ok(self.clone());
        }
        if players.iter().any(|player| player.id == *user_id) {
            return Ok(self.clone());
        }
        let tournament_user = TournamentUser::new(self.id, *user_id);
        tournament_user.insert(pool).await?;
        Ok(self.clone())
    }

    pub async fn leave(&self, user_id: &Uuid, pool: &DbPool) -> Result<Self, DbError> {
        TournamentUser::delete(self.id, *user_id, pool).await?;
        Ok(self.clone())
    }

    pub async fn from_uuid(uuid: &Uuid, pool: &DbPool) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(tournaments::table.find(uuid).first(connection).await?)
    }

    pub async fn from_nanoid(nano: &String, pool: &DbPool) -> Result<Tournament, DbError> {
        let connection = &mut get_conn(pool).await?;
        Ok(tournaments::table
            .filter(nanoid_field.eq(nano))
            .first(connection)
            .await?)
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

    pub async fn start(&self, _pool: &DbPool) -> Result<Vec<Game>, DbError> {
        //let connection = &mut get_conn(pool).await?;
        Ok(Vec::new())
    }

    pub async fn get_all(pool: &DbPool) -> Result<Vec<Tournament>, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(tournaments::table.get_results(conn).await?)
    }

    pub async fn find_by_nanoid(u: &str, pool: &DbPool) -> Result<Tournament, DbError> {
        let conn = &mut get_conn(pool).await?;
        Ok(tournaments::table
            .filter(nanoid_field.eq(u))
            .first(conn)
            .await?)
    }
}
