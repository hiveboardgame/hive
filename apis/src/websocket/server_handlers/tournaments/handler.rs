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
    websocket::{messages::HandlerOutput, WebsocketData, WsHub},
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
    pub hub: Arc<WsHub>,
}

impl TournamentHandler {
    pub async fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
            data,
            hub,
        })
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let output: HandlerOutput = match self.action.clone() {
            TournamentAction::Create(details) => {
                CreateHandler::new(*details, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Join(tournament_id) => {
                JoinHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Leave(tournament_id) => {
                LeaveHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Delete(tournament_id) => {
                DeleteHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationCreate(tournament_id, user) => {
                InvitationCreate::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationAccept(tournament_id) => {
                InvitationAccept::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationDecline(tournament_id) => {
                InvitationDecline::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::InvitationRetract(tournament_id, user) => {
                InvitationRetract::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Kick(tournament_id, user) => {
                KickHandler::new(tournament_id, self.user_id, user, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Start(tournament_id) => {
                StartHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::AdjudicateResult(game_id, new_result) => {
                AdjudicateResultHandler::new(game_id, new_result, self.user_id, &self.pool)
                    .await?
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
                .await?
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
                    .into()
            }
            TournamentAction::ProgressToNextRound(tournament_id) => {
                SwissRoundHandler::new(tournament_id, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
                    .into()
            }
        };
        // Invalidate cached recipient set when an action changes membership
        // or finalizes the tournament. Without this, dispatch can serve up
        // to TOURNAMENT_MEMBERS_TTL (5s) of stale fanout — e.g., a freshly
        // joined user missing tournament chat. Read paths rebuild on next
        // dispatch.
        // Exhaustive match: adding a new `TournamentAction` variant becomes a
        // compile error here instead of a silent stale-cache bug. The two
        // arms below partition the enum into "mutates membership" vs
        // "doesn't"; new variants should be added to whichever applies.
        let invalidate_id = match &self.action {
            TournamentAction::Join(id)
            | TournamentAction::Leave(id)
            | TournamentAction::Delete(id)
            | TournamentAction::InvitationAccept(id)
            | TournamentAction::Start(id)
            | TournamentAction::Finish(id)
            | TournamentAction::Abandon(id) => Some(id),
            TournamentAction::Kick(id, _) => Some(id),
            // No-op for the cache: these actions don't change the
            // players ∪ organizers set. Adjudication updates game
            // results; Create/InvitationCreate/Decline/Retract/
            // ProgressToNextRound touch tournament state but not
            // membership.
            TournamentAction::AdjudicateResult(_, _)
            | TournamentAction::DoubleForfeitUnstartedGames(_)
            | TournamentAction::ResetAdjudicatedGames(_)
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
