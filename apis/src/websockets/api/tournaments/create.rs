use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    models::{NewTournament, Tournament},
    DbPool,
};
use shared_types::TournamentDetails;
use uuid::Uuid;

pub struct CreateHandler {
    details: TournamentDetails,
    user_id: Uuid,
    pool: DbPool,
}

impl CreateHandler {
    pub async fn new(details: TournamentDetails, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            details,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let new_tournament = NewTournament::new(self.details.clone())?;
        let tournament = Tournament::create(self.user_id, &new_tournament, &self.pool).await?;
        let response = TournamentResponse::from_model(&tournament, &self.pool).await?;

        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Created(response)),
        }])
    }
}
