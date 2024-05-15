use crate::websockets::internal_server_message::{InternalServerMessage, MessageDestination};
use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
};
use anyhow::Result;
use db_lib::{models::Challenge, DbPool};
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
        let mut responses = Vec::new();
        for challenge in Challenge::get_public(&self.pool).await? {
            responses.push(ChallengeResponse::from_model(&challenge, &self.pool).await?);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Challenge(ChallengeUpdate::Challenges(responses)),
        }])
    }
}
