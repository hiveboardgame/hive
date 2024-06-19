use crate::{
    db_error::DbError, models::{tournament::Tournament, user::User}, schema::tournaments_invitations::{
        self, dsl::tournaments_invitations as tournaments_invitations_table,
    }, DbConn
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Identifiable, Insertable, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = invitee_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_invitations)]
#[diesel(primary_key(tournament_id, invitee_id))]
pub struct TournamentInvitation {
    pub tournament_id: Uuid,
    pub invitee_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl TournamentInvitation {
    pub fn new(tournament_id: Uuid, invitee_id: Uuid) -> Self {
        Self {
            tournament_id,
            invitee_id,
            created_at: Utc::now(),
        }
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournaments_invitations_table)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::delete(self).execute(conn).await?;
        Ok(())
    }
}
