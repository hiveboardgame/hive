use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use uuid::Uuid;

pub struct GetHandler {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl GetHandler {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_nanoid(&self.nanoid, &mut conn).await?;
        let tournament_response = TournamentResponse::from_model(&tournament, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Tournaments(vec![
                tournament_response,
            ])),
        }])
    }
}
