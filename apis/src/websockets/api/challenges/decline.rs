use crate::{
    common::{
        challenge_action::ChallengeVisibility,
        server_result::{
            ChallengeUpdate, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    responses::challenge::ChallengeResponse,
};
use shared_types::challenge_error::ChallengeError;
use anyhow::Result;
use db_lib::{models::challenge::Challenge, DbPool};
use uuid::Uuid;

pub struct DeclineHandler {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl DeclineHandler {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let challenge = Challenge::find_by_nanoid(&self.nanoid, &self.pool).await?;
        if challenge.opponent_id != Some(self.user_id) {
            return Err(ChallengeError::NotUserChallenge.into());
        }
        let challenge_response = ChallengeResponse::from_model(&challenge, &self.pool).await?;
        challenge.delete(&self.pool).await?;
        let mut messages = Vec::new();
        match challenge_response.visibility {
            ChallengeVisibility::Public => {
                unreachable!();
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Direct(challenge_response.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        challenge_response.nanoid,
                    )),
                });
            }
            ChallengeVisibility::Direct => {
                if let Some(opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge_response.nanoid.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge_response.nanoid,
                        )),
                    });
                }
            }
        }
        Ok(messages)
    }
}
