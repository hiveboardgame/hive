use crate::websockets::internal_server_message::{InternalServerMessage, MessageDestination};
use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
};
use anyhow::Result;
use db_lib::get_conn;
use db_lib::{models::Challenge, DbPool};
use shared_types::{ChallengeError, ChallengeId, ChallengeVisibility};
use uuid::Uuid;

pub struct GetHandler {
    challenge_id: ChallengeId,
    user_id: Uuid,
    pool: DbPool,
}

impl GetHandler {
    pub async fn new(challenge_id: ChallengeId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            challenge_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let challenge = Challenge::find_by_challenge_id(&self.challenge_id, &mut conn).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        if challenge.visibility == ChallengeVisibility::Public.to_string()
            || challenge.challenger_id == self.user_id
            || challenge.opponent_id == Some(self.user_id)
            || challenge.visibility == ChallengeVisibility::Private.to_string()
        {
            return Ok(vec![InternalServerMessage {
                destination: MessageDestination::User(challenge_response.challenger.uid),
                message: ServerMessage::Challenge(ChallengeUpdate::Challenges(vec![
                    challenge_response,
                ])),
            }]);
        }
        Err(ChallengeError::NotUserChallenge)?
    }
}
