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

pub struct DeclineHandler {
    challenge_id: ChallengeId,
    user_id: Uuid,
    pool: DbPool,
}

impl DeclineHandler {
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
        if challenge.opponent_id != Some(self.user_id) {
            return Err(ChallengeError::NotUserChallenge.into());
        }
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        challenge.delete(&mut conn).await?;
        let mut messages = Vec::new();
        match challenge_response.visibility {
            ChallengeVisibility::Public => {
                unreachable!();
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenge_response.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        challenge_response.challenge_id,
                    )),
                });
            }
            ChallengeVisibility::Direct => {
                if let Some(opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge_response.challenge_id.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge_response.challenge_id,
                        )),
                    });
                }
            }
        }
        Ok(messages)
    }
}
