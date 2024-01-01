use crate::{
    common::{
        challenge_action::ChallengeVisibility,
        challenge_error::ChallengeError,
        server_result::{
            ChallengeUpdate, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    responses::challenge::ChallengeResponse,
};
use anyhow::Result;
use db_lib::{models::challenge::Challenge, DbPool};
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
        let challenge = Challenge::find_by_nanoid(&self.nanoid, &self.pool).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &self.pool).await?;
        if challenge.visibility == ChallengeVisibility::Public.to_string()
            || challenge.challenger_id == self.user_id
            || challenge.opponent_id == Some(self.user_id)
            || challenge.visibility == ChallengeVisibility::Private.to_string()
        {
            return Ok(vec![InternalServerMessage {
                destination: MessageDestination::Direct(challenge_response.challenger.uid),
                message: ServerMessage::Challenge(ChallengeUpdate::Challenges(vec![
                    challenge_response,
                ])),
            }]);
        }
        Err(ChallengeError::NotUserChallenge)?
    }
}
