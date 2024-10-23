use super::accept::AcceptHandler;
use super::create::CreateHandler;
use super::delete::DeleteHandler;
use super::get::GetHandler;
use super::get_own::GetOwnHandler;
use super::get_public::GetPublicHandler;
use crate::common::ChallengeAction;
use crate::websocket::messages::InternalServerMessage;
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
    username: String,
}

impl ChallengeHandler {
    pub async fn new(
        action: ChallengeAction,
        username: &str,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
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
                DeleteHandler::new(challenge_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Decline(challenge_id) => {
                DeleteHandler::new(challenge_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Get(challenge_id) => {
                GetHandler::new(challenge_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::GetPublic => {
                GetPublicHandler::new(self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::GetOwn => {
                GetOwnHandler::new(self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            _ => unimplemented!(),
        };
        Ok(messages)
    }
}
