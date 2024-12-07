use super::{
    abandon::AbandonHandler, adjudicate_result::AdjudicateResultHandler, create::CreateHandler,
    delete::DeleteHandler, finish::FinishHandler, get::GetHandler, get_all::GetAllHandler,
    invitation_accept::InvitationAccept, invitation_create::InvitationCreate,
    invitation_decline::InvitationDecline, invitation_retract::InvitationRetract,
    join::JoinHandler, kick::KickHandler, leave::LeaveHandler, start::StartHandler,
};
use crate::{
    common::TournamentAction,
    websocket::{chat::Chats, messages::InternalServerMessage},
};
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub pool: DbPool,
    pub user_id: Uuid,
    pub username: String,
    pub chat_storage: actix_web::web::Data<Chats>,
}

impl TournamentHandler {
    pub async fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        chat_storage: actix_web::web::Data<Chats>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
            chat_storage,
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
            TournamentAction::Join(tournament_id) => {
                JoinHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Get(tournament_id, _depth) => {
                GetHandler::new(
                    tournament_id,
                    self.user_id,
                    self.chat_storage.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            TournamentAction::GetAll(depth, _) => {
                GetAllHandler::new(self.user_id, depth, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Leave(tournament_id) => {
                LeaveHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Delete(tournament_id) => {
                DeleteHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationCreate(tournament_id, user) => {
                InvitationCreate::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationAccept(tournament_id) => {
                InvitationAccept::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationDecline(tournament_id) => {
                InvitationDecline::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationRetract(tournament_id, user) => {
                InvitationRetract::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Kick(tournament_id, user) => {
                KickHandler::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Start(tournament_id) => {
                StartHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::AdjudicateResult(game_id, new_result) => {
                AdjudicateResultHandler::new(game_id, new_result, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Abandon(tournament_id) => {
                AbandonHandler::new(
                    tournament_id,
                    self.user_id,
                    self.username.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            TournamentAction::Finish(tournament_id) => {
                FinishHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
        };
        Ok(messages)
    }
}
