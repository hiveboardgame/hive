use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{models::Tournament, DbPool};
use uuid::Uuid;

pub struct LeaveHandler {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl LeaveHandler {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        // TODO: This needs to go into a one commit
        let tournament = Tournament::from_nanoid(&self.nanoid, &self.pool).await?;
        tournament.leave(&self.user_id, &self.pool).await?;
        let response = TournamentResponse::from_model(&tournament, &self.pool).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Left(response)),
        }])
    }
}
