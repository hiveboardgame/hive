use super::{append_public_tournament_patches, PublicTournamentSection};
use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, helpers::run_serializable, models::Tournament, DbPool};
use shared_types::TournamentId;
use uuid::Uuid;

pub struct JoinHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl JoinHandler {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament_id = self.tournament_id.clone();
        let user_id = self.user_id;
        let tournament = run_serializable(&mut conn, move |tc| {
            let tournament_id = tournament_id.clone();
            Box::pin(async move {
                let tournament =
                    Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
                tournament.join(&user_id, tc).await
            })
        })
        .await?;
        let response = TournamentId(tournament.nanoid.clone());
        let mut output = HandlerOutput::from(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(TournamentUpdate::Joined(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(response)),
            },
        ]);
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
