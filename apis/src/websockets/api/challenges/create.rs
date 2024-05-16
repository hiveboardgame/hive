use std::str::FromStr;

use anyhow::Result;
use db_lib::{
    models::{Challenge, NewChallenge, User},
    DbPool,
};
use shared_types::{ChallengeDetails, ChallengeVisibility};
use uuid::Uuid;

use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};

pub struct CreateHandler {
    details: ChallengeDetails,
    user_id: Uuid,
    pool: DbPool,
}
impl CreateHandler {
    pub async fn new(details: ChallengeDetails, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            details,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let opponent = match &self.details.opponent {
            Some(username) => Some((User::find_by_username(username, &self.pool).await?).id),
            None => None,
        };

        let new_challenge = NewChallenge::new(self.user_id, opponent, &self.details)?;

        let challenge = Challenge::create(&new_challenge, &self.pool).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &self.pool).await?;
        let mut messages = Vec::new();
        match ChallengeVisibility::from_str(&new_challenge.visibility)? {
            ChallengeVisibility::Direct => {
                if let Some(ref opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response,
                        )),
                    });
                }
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenge_response.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(challenge_response)),
                });
            }
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Created(challenge_response)),
                });
            }
        }
        Ok(messages)
    }
}
