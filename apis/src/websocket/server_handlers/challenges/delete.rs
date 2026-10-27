use crate::{
    common::{ChallengeUpdate, ServerMessage},
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
    pub fn new(challenge_id: ChallengeId, user_id: Uuid, admin: bool, pool: &DbPool) -> Self {
        Self {
            challenge_id,
            user_id,
            admin,
            pool: pool.clone(),
        }
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
        let visibility = challenge.visibility.parse::<ChallengeVisibility>()?;
        let challenge_id = ChallengeId(challenge.nanoid.clone());
        let challenger_id = challenge.challenger_id;
        let opponent_id = challenge.opponent_id;
        challenge.delete(&mut conn).await?;
        let mut messages = Vec::new();
        match visibility {
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
                });
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenger_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
                });
            }
            ChallengeVisibility::Direct => {
                if let Some(opponent_id) = opponent_id {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent_id),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge_id.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenger_id),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
                    });
                }
            }
        }
        Ok(messages)
    }
}
