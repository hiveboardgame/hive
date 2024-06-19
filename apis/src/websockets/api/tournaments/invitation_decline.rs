use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use uuid::Uuid;

pub struct InvitationDecline {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl InvitationDecline {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    let tournament = Tournament::from_nanoid(&self.nanoid, tc).await?;
                    Ok(tournament.decline_invitation(&self.user_id, tc).await?)
                }
                .scope_boxed()
            })
            .await?;

        let response = TournamentResponse::from_model(&tournament, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Joined(response)),
        }])
    }
}
