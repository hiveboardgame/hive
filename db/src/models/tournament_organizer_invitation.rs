use crate::{
    db_error::DbError,
    models::{Tournament, TournamentOrganizer, User},
    schema::{
        tournaments,
        tournaments_organizer_invitations,
        tournaments_organizers,
        tournaments_users,
        users,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

/// Only pending invitations are stored. Their lifetime belongs to the tournament,
/// so an inviter leaving cannot invalidate a different person's invitation.
#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = invitee_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_organizer_invitations)]
#[diesel(primary_key(tournament_id, invitee_id))]
pub struct TournamentOrganizerInvitation {
    pub tournament_id: Uuid,
    pub invitee_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl TournamentOrganizerInvitation {
    pub async fn find_by_user(user_id: Uuid, conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        Ok(tournaments_organizer_invitations::table
            .inner_join(tournaments::table)
            .filter(tournaments_organizer_invitations::invitee_id.eq(user_id))
            .filter(tournaments::finished_at.is_null())
            .select(Self::as_select())
            .load(conn)
            .await?)
    }
}

impl Tournament {
    /// Call under the tournament row lock, within the caller's transaction.
    pub async fn invite_organizer(
        &self,
        actor: Uuid,
        invitee: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        self.ensure_organizer_changes_open()?;
        self.ensure_user_is_organizer_or_admin(&actor, conn).await?;
        User::find_active_by_uuid(&invitee, conn).await?;
        if diesel::select(exists(
            tournaments_organizers::table.find((self.id, invitee)),
        ))
        .get_result::<bool>(conn)
        .await?
        {
            return Err(DbError::InvalidAction {
                info: String::from("This user is already an organizer"),
            });
        }
        Ok(
            diesel::insert_into(tournaments_organizer_invitations::table)
                .values(TournamentOrganizerInvitation {
                    tournament_id: self.id,
                    invitee_id: invitee,
                    created_at: Utc::now(),
                })
                .on_conflict_do_nothing()
                .execute(conn)
                .await?
                > 0,
        )
    }

    /// Acceptance never changes the separate player membership.
    pub async fn accept_organizer_invitation(
        &self,
        invitee: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        self.ensure_organizer_changes_open()?;
        User::find_active_by_uuid(&invitee, conn).await?;
        self.consume_organizer_invitation(invitee, conn).await?;
        diesel::insert_into(tournaments_organizers::table)
            .values(TournamentOrganizer::new(self.id, invitee))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn decline_organizer_invitation(
        &self,
        invitee: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        self.ensure_organizer_changes_open()?;
        User::find_active_by_uuid(&invitee, conn).await?;
        self.consume_organizer_invitation(invitee, conn).await
    }

    pub async fn retract_organizer_invitation(
        &self,
        actor: Uuid,
        invitee: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        self.ensure_organizer_changes_open()?;
        self.ensure_user_is_organizer_or_admin(&actor, conn).await?;
        self.consume_organizer_invitation(invitee, conn).await
    }

    async fn consume_organizer_invitation(
        &self,
        invitee: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        if diesel::delete(tournaments_organizer_invitations::table.find((self.id, invitee)))
            .execute(conn)
            .await?
            == 0
        {
            return Err(DbError::InvalidAction {
                info: String::from("No pending organizer invitation found"),
            });
        }
        Ok(())
    }

    /// Call under the same tournament lock as acceptance and account deletion.
    pub async fn leave_organizers(
        &self,
        actor: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        self.ensure_organizer_changes_open()?;
        User::find_active_by_uuid(&actor, conn).await?;
        let organizers = self.organizers(conn).await?;
        if !organizers.iter().any(|user| user.id == actor) {
            return Err(DbError::Unauthorized);
        }
        if !organizers.iter().any(|user| user.id != actor) {
            return Err(DbError::InvalidAction {
                info: String::from("The last organizer cannot leave"),
            });
        }
        diesel::delete(tournaments_organizers::table.find((self.id, actor)))
            .execute(conn)
            .await?;
        if self
            .start_setup()
            .is_some_and(|setup| setup.owner_id == actor)
        {
            diesel::update(tournaments::table.find(self.id))
                .set(tournaments::start_setup.eq(None::<serde_json::Value>))
                .execute(conn)
                .await?;
        }
        Ok(())
    }

    pub async fn retains_chat_access(
        &self,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        if User::find_active_by_uuid(&user_id, conn).await?.admin {
            return Ok(true);
        }
        let player = diesel::select(exists(tournaments_users::table.find((self.id, user_id))))
            .get_result::<bool>(conn)
            .await?;
        let organizer = diesel::select(exists(
            tournaments_organizers::table
                .inner_join(users::table)
                .filter(tournaments_organizers::tournament_id.eq(self.id))
                .filter(tournaments_organizers::organizer_id.eq(user_id))
                .filter(users::deleted.eq(false)),
        ))
        .get_result::<bool>(conn)
        .await?;
        Ok(player || organizer)
    }
}
