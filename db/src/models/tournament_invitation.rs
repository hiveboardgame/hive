use crate::{
    db_error::DbError,
    models::{tournament::Tournament, user::User},
    schema::tournaments_invitations::{
        self,
        dsl::{
            invitee_id as invitee_id_column,
            tournament_id as tournament_id_column,
            tournaments_invitations as tournaments_invitations_table,
        },
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select, Identifiable, Insertable, Queryable};
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
    /// Set when the invitee says no. The row is kept rather than deleted so an
    /// organizer can tell a decline apart from an invitation nobody has opened.
    pub declined_at: Option<DateTime<Utc>>,
}

impl TournamentInvitation {
    pub fn new(tournament_id: Uuid, invitee_id: Uuid) -> Self {
        Self {
            tournament_id,
            invitee_id,
            created_at: Utc::now(),
            declined_at: None,
        }
    }

    /// Records a decline without losing the invitation. Re-inviting is
    /// `insert` with `on_conflict`, which clears it again.
    pub async fn decline(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(self)
            .set(tournaments_invitations::declined_at.eq(Some(Utc::now())))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn declined_for_tournament(
        tournament: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        Ok(tournaments_invitations_table
            .filter(tournament_id_column.eq(tournament))
            .filter(tournaments_invitations::declined_at.is_not_null())
            .select(invitee_id_column)
            .get_results(conn)
            .await?)
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournaments_invitations_table)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn find_by_user(
        user: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<TournamentInvitation>, DbError> {
        Ok(tournaments_invitations_table
            .filter(invitee_id_column.eq(user))
            .get_results(conn)
            .await?)
    }

    pub async fn find_by_ids(
        t_id: &Uuid,
        i_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentInvitation, DbError> {
        Ok(tournaments_invitations_table
            .find((t_id, i_id))
            .first::<TournamentInvitation>(conn)
            .await?)
    }

    pub async fn delete(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::delete(self).execute(conn).await?;
        Ok(())
    }

    /// Whether an invitation is *outstanding* — a declined one no longer opens
    /// the door, which is how it behaved when declining deleted the row.
    pub async fn exists(t_id: &Uuid, i_id: &Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(select(exists(
            tournaments_invitations_table
                .filter(
                    tournament_id_column
                        .eq(t_id)
                        .and(invitee_id_column.eq(i_id)),
                )
                .filter(tournaments_invitations::declined_at.is_null()),
        ))
        .get_result(conn)
        .await?)
    }
}
