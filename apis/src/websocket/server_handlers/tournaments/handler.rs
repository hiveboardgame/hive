use std::sync::Arc;

use super::{
    abandon::AbandonHandler,
    adjudicate_result::AdjudicateResultHandler,
    bulk_adjudicate::{BulkAdjudicateHandler, BulkAdjudication},
    create::CreateHandler,
    delete::DeleteHandler,
    finish::FinishHandler,
    invitation_accept::InvitationAccept,
    invitation_create::InvitationCreate,
    invitation_decline::InvitationDecline,
    invitation_retract::InvitationRetract,
    join::JoinHandler,
    kick::KickHandler,
    leave::LeaveHandler,
    progress_to_next_round::SwissRoundHandler,
    start::StartHandler,
};
use crate::{
    common::TournamentAction,
    websocket::{messages::HandlerOutput, WsHub},
};
use anyhow::Result;
use db_lib::DbPool;
use uuid::Uuid;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub pool: DbPool,
    pub user_id: Uuid,
    pub username: String,
    pub hub: Arc<WsHub>,
}

impl TournamentHandler {
    pub fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
            hub,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let output: HandlerOutput = match self.action.clone() {
            TournamentAction::Create(details) => {
                CreateHandler::new(*details, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Join(tournament_id) => {
                JoinHandler::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Leave(tournament_id) => {
                let output = LeaveHandler::new(tournament_id.clone(), self.user_id, &self.pool)
                    .handle()
                    .await?;
                self.hub
                    .unsubscribe_user_from_tournament_chat(self.user_id, &tournament_id);
                output.into()
            }
            TournamentAction::Delete(tournament_id) => {
                DeleteHandler::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationCreate(tournament_id, user) => {
                InvitationCreate::new(tournament_id, self.user_id, user, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationAccept(tournament_id) => {
                InvitationAccept::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationDecline(tournament_id) => {
                InvitationDecline::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationRetract(tournament_id, user) => {
                InvitationRetract::new(tournament_id, self.user_id, user, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Kick(tournament_id, user) => {
                let output =
                    KickHandler::new(tournament_id.clone(), self.user_id, user, &self.pool)
                        .handle()
                        .await?;
                self.hub
                    .unsubscribe_user_from_tournament_chat(user, &tournament_id);
                output.into()
            }
            TournamentAction::Start(tournament_id) => {
                StartHandler::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::AdjudicateResult(game_id, new_result) => {
                AdjudicateResultHandler::new(game_id, new_result, self.user_id, &self.pool)
                    .handle()
                    .await?
            }
            TournamentAction::DoubleForfeitUnstartedGames(tournament_id) => {
                BulkAdjudicateHandler::new(
                    tournament_id,
                    self.user_id,
                    BulkAdjudication::DoubleForfeitUnstarted,
                    &self.pool,
                )
                .handle()
                .await?
            }
            TournamentAction::ResetAdjudicatedGames(tournament_id) => {
                BulkAdjudicateHandler::new(
                    tournament_id,
                    self.user_id,
                    BulkAdjudication::ResetAdjudicated,
                    &self.pool,
                )
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
                .handle()
                .await?
            }
            TournamentAction::Finish(tournament_id) => {
                FinishHandler::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::ProgressToNextRound(tournament_id) => {
                SwissRoundHandler::new(tournament_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
        };
        // Invalidate cached recipients when an action changes membership or
        // deletes the tournament. The next dispatch rebuilds the entry.
        // Exhaustive match: adding a new `TournamentAction` variant becomes a
        // compile error here instead of a silent stale-cache bug. The two
        // arms below partition the enum into "mutates membership" vs
        // "doesn't"; new variants should be added to whichever applies.
        let invalidate_id = match &self.action {
            TournamentAction::Join(id)
            | TournamentAction::Leave(id)
            | TournamentAction::Delete(id)
            | TournamentAction::InvitationAccept(id) => Some(id),
            TournamentAction::Kick(id, _) => Some(id),
            // These actions change invitations, games, or tournament state,
            // but not the players ∪ organizers recipient set.
            TournamentAction::AdjudicateResult(_, _)
            | TournamentAction::Abandon(_)
            | TournamentAction::DoubleForfeitUnstartedGames(_)
            | TournamentAction::Finish(_)
            | TournamentAction::ResetAdjudicatedGames(_)
            | TournamentAction::Start(_)
            | TournamentAction::Create(_)
            | TournamentAction::InvitationCreate(_, _)
            | TournamentAction::InvitationDecline(_)
            | TournamentAction::InvitationRetract(_, _)
            | TournamentAction::ProgressToNextRound(_) => None,
        };
        if let Some(id) = invalidate_id {
            self.hub.invalidate_tournament_members(id);
        }
        Ok(output)
    }
}
