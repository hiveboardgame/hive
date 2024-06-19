use crate::{
    db_error::DbError,
    models::{tournament_series::TournamentSeries, user::User},
    schema::tournament_series_organizers::{
        self, dsl::tournament_series_organizers as tournament_series_organizers_table,
    },
    DbConn,
};
use diesel::{prelude::*, Identifiable, Insertable, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = organizer_id))]
#[diesel(belongs_to(TournamentSeries))]
#[diesel(table_name = tournament_series_organizers)]
#[diesel(primary_key(tournament_series_id, organizer_id))]
pub struct TournamentSeriesOrganizer {
    pub tournament_series_id: Uuid,
    pub organizer_id: Uuid,
}

impl TournamentSeriesOrganizer {
    pub fn new(tournament_series_id: Uuid, organizer_id: Uuid) -> Self {
        Self {
            tournament_series_id,
            organizer_id,
        }
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournament_series_organizers_table)
            .execute(conn)
            .await?;
        Ok(())
    }
}
