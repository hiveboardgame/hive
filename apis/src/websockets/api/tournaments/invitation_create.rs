use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    models::{Tournament, TournamentInvitation},
    DbPool,
};
use uuid::Uuid;

pub struct InvitationCreate {
    nanoid: String,
    user_id: Uuid,
    invitee: Uuid,
    pool: DbPool,
}

impl InvitationCreate {
    pub async fn new(nanoid: String, user_id: Uuid, invitee: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            invitee,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        // TODO: This needs to go into a one commit
        let tournament = Tournament::from_nanoid(&self.nanoid, &self.pool).await?;
        let invitation = TournamentInvitation::new(tournament.id, self.invitee);
        invitation.insert(&self.pool).await?;
        // TODO: @leex needs to send the invitation as well
        let response = TournamentResponse::from_model(&tournament, &self.pool).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Joined(response)),
        }])
    }
}
