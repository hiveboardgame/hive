use crate::{
    db_error::DbError,
    models::{tournament::Tournament, user::User},
    schema::tournaments_users::{self, dsl::tournaments_users as tournament_user_table},
    DbConn,
};
use diesel::{prelude::*, Identifiable, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_users)]
#[diesel(primary_key(tournament_id, user_id))]
pub struct TournamentUser {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
}

impl TournamentUser {
    pub fn new(tournament_id: Uuid, user_id: Uuid) -> Self {
        Self {
            tournament_id,
            user_id,
        }
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournament_user_table)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete(
        tournament_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::delete(tournaments_users::table.find((tournament_id, user_id)))
            .execute(conn)
            .await?;
        Ok(())
    }
}
