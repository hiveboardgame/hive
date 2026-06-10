use super::{accept::AcceptHandler, create::CreateHandler, delete::DeleteHandler};
use crate::{
    common::{ChallengeAction, ServerMessage},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::DbPool;
use shared_types::ChallengeVisibility;
use uuid::Uuid;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
    admin: bool,
    guest: bool,
    username: String,
}

impl ChallengeHandler {
    pub async fn new(
        action: ChallengeAction,
        username: &str,
        user_id: Uuid,
        admin: bool,
        guest: bool,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
            admin,
            guest,
            username: username.to_owned(),
        })
    }

    fn error(&self, msg: &str) -> Vec<InternalServerMessage> {
        vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Error(msg.to_string()),
        }]
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create(mut details) => {
                // Guests play casual only, via shareable links — no rated games
                // and no public lobby.
                if self.guest {
                    details.rated = false;
                    if details.visibility == ChallengeVisibility::Public {
                        return Ok(self.error(
                            "Guests can only create private or direct challenges. Register to use the public lobby.",
                        ));
                    }
                }
                CreateHandler::new(details, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Accept(challenge_id) => {
                AcceptHandler::new(
                    challenge_id,
                    &self.username,
                    self.user_id,
                    self.guest,
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ChallengeAction::Delete(challenge_id) => {
                DeleteHandler::new(challenge_id, self.user_id, self.admin, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::DeleteMany(ids) => {
                let mut messages = Vec::new();
                for challenge_id in ids {
                    let mut msgs =
                        DeleteHandler::new(challenge_id, self.user_id, self.admin, &self.pool)
                            .await?
                            .handle()
                            .await?;
                    messages.append(&mut msgs);
                }
                messages
            }
        };
        Ok(messages)
    }
}
