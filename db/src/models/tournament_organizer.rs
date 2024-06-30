use crate::{
    db_error::DbError,
    get_conn,
    models::{tournament::Tournament, user::User},
    schema::tournaments_organizers::{
        self, dsl::tournaments_organizers as tournament_organizer_table,
    },
    DbPool,
};
use diesel::{prelude::*, Identifiable, Insertable, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = organizer_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_organizers)]
#[diesel(primary_key(tournament_id, organizer_id))]
pub struct TournamentOrganizer {
    pub tournament_id: Uuid,
    pub organizer_id: Uuid,
}

impl TournamentOrganizer {
    pub fn new(tournament_id: Uuid, organizer_id: Uuid) -> Self {
        Self {
            tournament_id,
            organizer_id,
        }
    }

    pub async fn insert(&self, pool: &DbPool) -> Result<(), DbError> {
        let conn = &mut get_conn(pool).await?;
        self.insert_into(tournament_organizer_table)
            .execute(conn)
            .await?;
        Ok(())
    }
}
