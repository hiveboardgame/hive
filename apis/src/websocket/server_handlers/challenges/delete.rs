use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Challenge, DbPool};
use shared_types::{ChallengeError, ChallengeId, ChallengeVisibility};
use uuid::Uuid;

pub struct DeleteHandler {
    challenge_id: ChallengeId,
    user_id: Uuid,
    admin: bool,
    pool: DbPool,
}

impl DeleteHandler {
    pub async fn new(
        challenge_id: ChallengeId,
        user_id: Uuid,
        admin: bool,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            challenge_id,
            user_id,
            admin,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let challenge = match Challenge::find_by_challenge_id(&self.challenge_id, &mut conn).await {
            Ok(challenge) => challenge,
            Err(DbError::NotFound { .. }) => {
                return Ok(vec![InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        self.challenge_id.clone(),
                    )),
                }]);
            }
            Err(err) => return Err(err.into()),
        };
        if !self.admin
            && challenge.challenger_id != self.user_id
            && challenge.opponent_id != Some(self.user_id)
        {
            return Err(ChallengeError::NotUserChallenge.into());
        }
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        challenge.delete(&mut conn).await?;
        let mut messages = Vec::new();
        match challenge_response.visibility {
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        challenge_response.challenge_id,
                    )),
                });
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
