use crate::{common::TournamentAction, websockets::internal_server_message::InternalServerMessage};
use anyhow::Result;
use db_lib::{
    models::{NewTournament, Tournament, User},
    DbPool,
};
use uuid::Uuid;

use super::create::CreateHandler;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub pool: DbPool,
    pub user_id: Uuid,
    pub username: String,
}

impl TournamentHandler {
    pub async fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.action.clone() {
            TournamentAction::Create(details) => {
                CreateHandler::new(*details, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            _ => unimplemented!(),
        };
        Ok(messages)
    }
}
