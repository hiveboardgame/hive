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

pub struct InvitationRetract {
    nanoid: String,
    user_id: Uuid,
    invitee: Uuid,
    pool: DbPool,
}

impl InvitationRetract {
    pub async fn new(nanoid: String, user_id: Uuid, invitee: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            invitee,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    let tournament = Tournament::from_nanoid(&self.nanoid, tc).await?;
                    Ok(tournament
                        .retract_invitation(&self.user_id, &self.invitee, tc)
                        .await?)
                }
                .scope_boxed()
            })
            .await?;
        let response = TournamentResponse::from_model(&tournament, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Modified(response)),
        }])
    }
}
