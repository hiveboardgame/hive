use super::get::GetHandler;
use super::get_own::GetOwnHandler;
use super::get_public::GetPublicHandler;
use crate::common::ChallengeAction;
use crate::websockets::api::challenges::accept::AcceptHandler;
use crate::websockets::api::challenges::create::CreateHandler;
use crate::websockets::api::challenges::delete::DeleteHandler;
use crate::websockets::internal_server_message::InternalServerMessage;
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
            ChallengeAction::Accept(nanoid) => {
                AcceptHandler::new(nanoid, &self.username, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Delete(nanoid) => {
                DeleteHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Decline(nanoid) => {
                DeleteHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Get(nanoid) => {
                GetHandler::new(nanoid, self.user_id, &self.pool)
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
