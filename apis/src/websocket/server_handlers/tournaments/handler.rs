use std::sync::Arc;

use super::{
    abandon::AbandonHandler, adjudicate_result::AdjudicateResultHandler, create::CreateHandler,
    delete::DeleteHandler, finish::FinishHandler, invitation_accept::InvitationAccept,
    invitation_create::InvitationCreate, invitation_decline::InvitationDecline,
    invitation_retract::InvitationRetract, join::JoinHandler, kick::KickHandler,
    leave::LeaveHandler, progress_to_next_round::SwissRoundHandler, start::StartHandler,
};
use crate::{
    common::TournamentAction,
    websocket::{messages::InternalServerMessage, WebsocketData},
};
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub pool: DbPool,
    pub user_id: Uuid,
    pub username: String,
    pub data: Arc<WebsocketData>,
}

impl TournamentHandler {
    pub async fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
            data,
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
            TournamentAction::ProgressToNextRound(tournament_id) => {
                SwissRoundHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
        };
        Ok(messages)
    }
}
