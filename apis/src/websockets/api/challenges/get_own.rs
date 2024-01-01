use crate::{
    common::server_result::{
        ChallengeUpdate, InternalServerMessage, MessageDestination, ServerMessage,
    },
    responses::challenge::ChallengeResponse,
};
use anyhow::Result;
use db_lib::{models::challenge::Challenge, DbPool};
use uuid::Uuid;

pub struct GetOwnHandler {
    user_id: Uuid,
    pool: DbPool,
}

impl GetOwnHandler {
    pub async fn new(user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut responses = Vec::new();
        for challenge in Challenge::get_own(self.user_id, &self.pool).await? {
            responses.push(ChallengeResponse::from_model(&challenge, &self.pool).await?);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Direct(self.user_id),
            message: ServerMessage::Challenge(ChallengeUpdate::Challenges(responses)),
        }])
    }
}
