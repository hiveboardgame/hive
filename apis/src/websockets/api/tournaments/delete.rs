use crate::{
    common::{ServerMessage, TournamentUpdate},
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use uuid::Uuid;

pub struct DeleteHandler {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl DeleteHandler {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut tournament = Tournament::from_nanoid(&self.nanoid, &mut conn).await?;
        tournament.delete(self.user_id, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Deleted(self.nanoid.clone())),
        }])
    }
}
