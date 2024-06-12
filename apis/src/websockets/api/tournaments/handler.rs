use super::{
    create::CreateHandler, delete::DeleteHandler, get::GetHandler, get_all::GetAllHandler, invitation_accept::InvitationAccept, invitation_create::InvitationCreate, invitation_decline::InvitationDecline, invitation_retract::InvitationRetract, join::JoinHandler, leave::LeaveHandler
};
use crate::{common::TournamentAction, websockets::internal_server_message::InternalServerMessage};
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

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
            TournamentAction::Join(nanoid) => {
                JoinHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Get(nanoid) => {
                GetHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::GetAll => {
                GetAllHandler::new(self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Leave(nanoid) => {
                LeaveHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Delete(nanoid) => {
                DeleteHandler::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationCreate(nanoid, user) => {
                InvitationCreate::new(nanoid, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationAccept(nanoid) => {
                InvitationAccept::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationDecline(nanoid) => {
                InvitationDecline::new(nanoid, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationRetract(nanoid, user) => {
                InvitationRetract::new(nanoid, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            _ => unimplemented!(),
        };
        Ok(messages)
    }
}
