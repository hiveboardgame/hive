use super::{append_public_tournament_patches, PublicTournamentSection};
use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct InvitationDecline {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl InvitationDecline {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                let tournament =
                    Tournament::find_by_tournament_id_for_update(&self.tournament_id, tc).await?;
                Ok(tournament.decline_invitation(&self.user_id, tc).await?)
            })
            .await?;

        let response = TournamentId(tournament.nanoid.clone());
        let mut output = HandlerOutput::from(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Declined(response)),
        }]);
        append_public_tournament_patches(
            tournament.id,
            &[PublicTournamentSection::Memberships],
            &mut output,
            &mut conn,
        )
        .await;
        Ok(output.messages)
    }
}
