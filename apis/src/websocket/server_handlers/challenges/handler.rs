use super::accept::AcceptHandler;
use super::create::CreateHandler;
use super::delete::DeleteHandler;
use super::get::GetHandler;
use super::get_own::GetOwnHandler;
use super::get_public::GetPublicHandler;
use crate::common::ChallengeAction;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::new_style::server::TabData;
use anyhow::Result;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    client: TabData
}

impl ChallengeHandler {
    pub async fn new(
        action: ChallengeAction,
        client:  TabData,
    ) -> Result<Self> {
        Ok(Self {
            challenge_action: action,
            client
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let (user_id, username) = self.client.account().map(|a| (a.id, a.username.clone())).unwrap_or_default();
        let pool = self.client.pool();
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create(details) => {
                CreateHandler::new(details, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Accept(challenge_id) => {
                AcceptHandler::new(challenge_id, &username, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Delete(challenge_id) => {
                DeleteHandler::new(challenge_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Decline(challenge_id) => {
                DeleteHandler::new(challenge_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::Get(challenge_id) => {
                GetHandler::new(challenge_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::GetPublic => {
                GetPublicHandler::new(user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            ChallengeAction::GetOwn => {
                GetOwnHandler::new(user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            _ => unimplemented!(),
        };
        Ok(messages)
    }
}
