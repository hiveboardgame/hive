use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Challenge, NewChallenge, User},
    DbPool,
};
use shared_types::{ChallengeDetails, ChallengeVisibility};
use std::str::FromStr;
use uuid::Uuid;

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
        let mut conn = get_conn(&self.pool).await?;
        let opponent = match &self.details.opponent {
            Some(username) => Some((User::find_by_username(username, &mut conn).await?).id),
            None => None,
        };

        let new_challenge =
            NewChallenge::new(self.user_id, opponent, &self.details, &mut conn).await?;
        let challenge = Challenge::create(&new_challenge, &mut conn).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
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
