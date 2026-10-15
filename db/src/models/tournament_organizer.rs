use crate::{
    db_error::DbError,
    models::{tournament::Tournament, user::User},
    schema::tournaments_organizers,
    DbConn,
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
    pub(crate) async fn exists(
        tournament_id: Uuid,
        organizer_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        Ok(diesel::select(diesel::dsl::exists(
            tournaments_organizers::table.find((tournament_id, organizer_id)),
        ))
        .get_result(conn)
        .await?)
    }

    pub fn new(tournament_id: Uuid, organizer_id: Uuid) -> Self {
        Self {
            tournament_id,
            organizer_id,
        }
    }
}
