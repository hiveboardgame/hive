use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{models::{Tournament, TournamentInvitation}, DbPool};
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
        // TODO: Implement this
        let tournamet = TournamentResponse::from_nanoid(&self.nanoid, &self.pool).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Modified(tournamet)),
        }])
    }
}
