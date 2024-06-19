use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Tournament, TournamentInvitation},
    DbPool,
};
use shared_types::TournamentId;
use uuid::Uuid;

pub struct InvitationCreate {
    tournament_id: TournamentId,
    user_id: Uuid,
    invitee: Uuid,
    pool: DbPool,
}

impl InvitationCreate {
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        invitee: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            invitee,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let invitation = TournamentInvitation::new(tournament.id, self.invitee);
        invitation.insert(&mut conn).await?;
        // TODO: @leex needs to send the invitation as well
        let response = TournamentResponse::from_model(&tournament, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Joined(response)),
        }])
    }
}
