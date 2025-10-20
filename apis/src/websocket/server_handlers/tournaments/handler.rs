use std::sync::Arc;

use super::{
    abandon::AbandonHandler, adjudicate_result::AdjudicateResultHandler, create::CreateHandler,
    delete::DeleteHandler, finish::FinishHandler, invitation_accept::InvitationAccept,
    invitation_create::InvitationCreate, invitation_decline::InvitationDecline,
    invitation_retract::InvitationRetract, join::JoinHandler, kick::KickHandler,
    leave::LeaveHandler, progress_to_next_round::SwissRoundHandler, start::StartHandler,
};
use crate::{
    common::TournamentAction, websocket::{messages::InternalServerMessage, new_style::server::{ServerData, TabData}}
};
use anyhow::Result;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub client: TabData,
    pub server: Arc<ServerData>
}

impl TournamentHandler {
    pub async fn new(
        action: TournamentAction,
        client: TabData,
        server: Arc<ServerData>
    ) -> Result<Self> {
        Ok(Self {
            client,
            server,
            action,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let (user_id, username) = self.client.account().map(|a| (a.id,a.username.clone())).unwrap_or_default();
        let pool = self.client.pool();
        let messages = match self.action.clone() {
            TournamentAction::Create(details) => {
                CreateHandler::new(*details, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Join(tournament_id) => {
                JoinHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Leave(tournament_id) => {
                LeaveHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Delete(tournament_id) => {
                DeleteHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationCreate(tournament_id, user) => {
                InvitationCreate::new(tournament_id, user_id, user, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationAccept(tournament_id) => {
                InvitationAccept::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationDecline(tournament_id) => {
                InvitationDecline::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::InvitationRetract(tournament_id, user) => {
                InvitationRetract::new(tournament_id, user_id, user, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Kick(tournament_id, user) => {
                KickHandler::new(tournament_id, user_id, user, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Start(tournament_id) => {
                StartHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::AdjudicateResult(game_id, new_result) => {
                AdjudicateResultHandler::new(game_id, new_result, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Abandon(tournament_id) => {
                AbandonHandler::new(
                    tournament_id,
                    user_id,
                    username.clone(),
                    pool,
                )
                .await?
                .handle()
                .await?
            }
            TournamentAction::Finish(tournament_id) => {
                FinishHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::ProgressToNextRound(tournament_id) => {
                SwissRoundHandler::new(tournament_id, user_id, pool)
                    .await?
                    .handle()
                    .await?
            }
            TournamentAction::Subscribe(tournament_id) =>{
                //Handle subscribe
                vec![]
            }
        };
        Ok(messages)
    }
}
