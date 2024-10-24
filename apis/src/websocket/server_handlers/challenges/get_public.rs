use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Challenge, DbPool};
use uuid::Uuid;

pub struct GetPublicHandler {
    user_id: Uuid,
    pool: DbPool,
}

impl GetPublicHandler {
    pub async fn new(user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut responses = Vec::new();
        for challenge in Challenge::get_public(&mut conn).await? {
            responses.push(ChallengeResponse::from_model(&challenge, &mut conn).await?);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Challenge(ChallengeUpdate::Challenges(responses)),
        }])
    }
}
