use super::{accept::AcceptHandler, create::CreateHandler, delete::DeleteHandler};
use crate::{
    common::{ChallengeAction, ServerMessage},
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::DbPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct ChallengeHandler {
    challenge_action: ChallengeAction,
    pool: DbPool,
    user_id: Uuid,
    admin: bool,
    data: Arc<WebsocketData>,
}

impl ChallengeHandler {
    pub fn new(
        action: ChallengeAction,
        user_id: Uuid,
        admin: bool,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            pool: pool.clone(),
            challenge_action: action,
            user_id,
            admin,
            data,
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.challenge_action.clone() {
            ChallengeAction::Create(details) => {
                self.data.realtime_gate.admit(details.time_mode)?;
                CreateHandler::new(details, self.user_id, &self.pool)
                    .handle()
                    .await?
            }
            ChallengeAction::Accept(challenge_id) => {
                AcceptHandler::new(challenge_id, self.user_id, &self.pool)
                    .handle(&self.data.realtime_gate)
                    .await?
            }
            ChallengeAction::Delete(challenge_id) => {
                DeleteHandler::new(challenge_id, self.user_id, self.admin, &self.pool)
                    .handle()
                    .await?
            }
            ChallengeAction::DeleteMany(ids) => {
                // Each delete commits on its own, so one failure must not swallow the
                // removals already made: their broadcasts still go out.
                let mut messages = Vec::new();
                let mut failed = Vec::new();
                for challenge_id in ids {
                    match DeleteHandler::new(
                        challenge_id.clone(),
                        self.user_id,
                        self.admin,
                        &self.pool,
                    )
                    .handle()
                    .await
                    {
                        Ok(mut removed) => messages.append(&mut removed),
                        Err(error) => {
                            log::warn!("Deleting challenge {challenge_id} failed: {error}");
                            failed.push(challenge_id.0);
                        }
                    }
                }
                if !failed.is_empty() {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(self.user_id),
                        message: ServerMessage::Error(format!(
                            "Could not delete challenges: {}",
                            failed.join(", ")
                        )),
                    });
                }
                messages
            }
        };
        Ok(messages)
    }
}
