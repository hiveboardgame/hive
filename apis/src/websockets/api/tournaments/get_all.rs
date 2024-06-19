use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use uuid::Uuid;

pub struct GetAllHandler {
    user_id: Uuid,
    pool: DbPool,
}

impl GetAllHandler {
    pub async fn new(user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournaments = Tournament::get_all(&mut conn).await?;
        let mut responses = Vec::new();
        for tournament in tournaments {
            let tournament_response =
                TournamentResponse::from_model(&tournament, &mut conn).await?;
            responses.push(tournament_response);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Tournaments(responses)),
        }])
    }
}
