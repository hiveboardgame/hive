use super::{accept::AcceptHandler, create::CreateHandler, delete::DeleteHandler};
use crate::{common::ChallengeAction, websocket::messages::InternalServerMessage};
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
    admin: bool,
    username: String,
}

impl ChallengeHandler {
    pub async fn new(
        action: ChallengeAction,
        username: &str,
        user_id: Uuid,
        admin: bool,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
            admin,
            username: username.to_owned(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create(details) => {
                CreateHandler::new(details, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Accept(challenge_id) => {
                AcceptHandler::new(challenge_id, &self.username, self.user_id, &self.pool)
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
