use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use shared_types::TournamentId;
use uuid::Uuid;

pub struct DeleteHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl DeleteHandler {
    pub async fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut tournament =
            Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        tournament.delete(self.user_id, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Deleted(
                self.tournament_id.clone(),
            )),
        }])
    }
}
