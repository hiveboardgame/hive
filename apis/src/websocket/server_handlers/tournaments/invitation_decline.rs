use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::TournamentId;
use uuid::Uuid;

async fn decline_invitation_tx(
    pool: &DbPool,
    tournament_id: TournamentId,
    user_id: Uuid,
) -> Result<db_lib::models::Tournament, DbError> {
    let mut conn = get_conn(pool).await?;
    conn.transaction::<_, DbError, _>(move |tc| {
        let tournament_id = tournament_id;
        let user_id = user_id;
        async move {
            let tournament =
                Tournament::find_by_tournament_id(&tournament_id, tc).await?;
            tournament.decline_invitation(&user_id, tc).await
        }
        .scope_boxed()
    })
    .await
}

pub struct InvitationDecline {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl InvitationDecline {
    pub async fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let tournament = decline_invitation_tx(
            &self.pool,
            self.tournament_id.clone(),
            self.user_id,
        )
        .await
        .map_err(anyhow::Error::from)?;

        let response = TournamentId(tournament.nanoid.clone());
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(TournamentUpdate::Declined(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::Modified(response)),
            },
        ])
    }
}
