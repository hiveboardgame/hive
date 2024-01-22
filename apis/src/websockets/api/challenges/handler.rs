use crate::common::challenge_action::ChallengeAction;
use crate::websockets::api::challenges::accept::AcceptHandler;
use crate::websockets::api::challenges::create::CreateHandler;
use crate::websockets::api::challenges::delete::DeleteHandler;
use crate::websockets::internal_server_message::InternalServerMessage;
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

use super::get::GetHandler;
use super::get_own::GetOwnHandler;
use super::get_public::GetPublicHandler;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
}

impl ChallengeHandler {
    pub async fn new(action: ChallengeAction, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create {
                rated,
                game_type,
                visibility,
                opponent,
                color_choice,
                time_mode,
                time_base,
                time_increment,
            } => {
                CreateHandler::new(
                    rated,
                    game_type,
                    visibility,
                    color_choice,
                    opponent,
                    time_mode,
                    time_base,
                    time_increment,
                    self.user_id,
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ChallengeAction::Accept(nanoid) => {
                AcceptHandler::new(nanoid, self.user_id, &self.pool)
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
