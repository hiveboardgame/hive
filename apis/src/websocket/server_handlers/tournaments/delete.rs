use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::{Error, Result};
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct DeleteHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl DeleteHandler {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        conn.transaction::<_, Error, _>(async move |tc| {
            let mut tournament =
                Tournament::find_by_tournament_id_for_update(&self.tournament_id, tc).await?;
            tournament.delete(self.user_id, tc).await?;
            Ok(())
        })
        .await?;
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::Deleted(
                    self.tournament_id.clone(),
                )),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(
                    self.tournament_id.clone(),
                )),
            },
        ])
    }
}
