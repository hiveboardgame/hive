use super::{
    accept::AcceptHandler,
    create::CreateHandler,
    delete::DeleteHandler,
    get::GetHandler,
    get_own::GetOwnHandler,
    get_public::GetPublicHandler,
};
use crate::{common::ChallengeAction, websocket::messages::InternalServerMessage};
use anyhow::Result;
use db_lib::DbPool;
use std::sync::{atomic::AtomicBool, Arc};
use uuid::Uuid;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    realtime_games_enabled: Arc<AtomicBool>,
}

impl ChallengeHandler {
    pub async fn new(
        action: ChallengeAction,
        username: &str,
        user_id: Uuid,
        pool: &DbPool,
        realtime_games_enabled: Arc<AtomicBool>,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
            username: username.to_owned(),
            realtime_games_enabled,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create(details) => {
                CreateHandler::new(
                    details,
                    self.user_id,
                    &self.pool,
                    Arc::clone(&self.realtime_games_enabled),
                )
                .await?
                .handle()
                .await?
            }
            ChallengeAction::Accept(challenge_id) => {
                AcceptHandler::new(
                    challenge_id,
                    &self.username,
                    self.user_id,
                    &self.pool,
                    Arc::clone(&self.realtime_games_enabled),
                )
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
            ChallengeAction::DeleteMany(ids) => {
                let mut messages = Vec::new();
                for challenge_id in ids {
                    let mut msgs = DeleteHandler::new(challenge_id, self.user_id, &self.pool)
                        .await?
                        .handle()
                        .await?;
                    messages.append(&mut msgs);
                }
                messages
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
