use std::sync::Arc;

use super::{
    adjudicate_result::AdjudicateResultHandler,
    arena::ArenaHandler,
    closeout::CloseoutHandler,
    create::CreateHandler,
    delete::DeleteHandler,
    invitation_accept::InvitationAccept,
    invitation_create::InvitationCreate,
    invitation_decline::InvitationDecline,
    invitation_retract::InvitationRetract,
    join::JoinHandler,
    kick::KickHandler,
    leave::LeaveHandler,
    organizers::{self, OrganizerOperation},
    start::StartHandler,
    withdraw::WithdrawHandler,
};
use crate::{
    common::TournamentAction,
    websocket::{
        messages::{HandlerOutput, SocketTx},
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use shared_types::TournamentId;
use uuid::Uuid;

pub struct TournamentHandler {
    pub action: TournamentAction,
    pub pool: DbPool,
    pub user_id: Uuid,
    pub username: String,
    pub received_from: SocketTx,
    pub hub: Arc<WsHub>,
}

impl TournamentHandler {
    pub fn new(
        action: TournamentAction,
        username: &str,
        user_id: Uuid,
        received_from: SocketTx,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            pool: pool.clone(),
            action,
            user_id,
            username: username.to_owned(),
            received_from,
            hub,
        }
    }

    async fn unsubscribe_if_no_membership(&self, user: Uuid, id: &TournamentId) {
        let membership = async {
            let mut conn = get_conn(&self.pool).await?;
            let tournament = Tournament::find_by_tournament_id(id, &mut conn).await?;
            tournament.retains_chat_access(user, &mut conn).await
        }
        .await;
        match membership {
            Ok(true) => (),
            Ok(false) => self.hub.unsubscribe_user_from_tournament_chat(user, id),
            Err(error) => {
                log::warn!("Tournament {id} membership changed but chat membership could not be reloaded: {error}");
                self.hub.unsubscribe_user_from_tournament_chat(user, id);
            }
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let output: HandlerOutput = match self.action.clone() {
            TournamentAction::Create(details) => CreateHandler::new(
                *details,
                self.user_id,
                self.received_from.clone(),
                &self.pool,
            )
            .handle()
            .await?
            .into(),
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
                self.unsubscribe_if_no_membership(self.user_id, &tournament_id)
                    .await;
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
            TournamentAction::OrganizerInvite(id, invitee) => {
                organizers::handle(
                    id,
                    self.user_id,
                    OrganizerOperation::Invite(invitee),
                    &self.pool,
                )
                .await?
            }
            TournamentAction::OrganizerAccept(id) => {
                organizers::handle(id, self.user_id, OrganizerOperation::Accept, &self.pool).await?
            }
            TournamentAction::OrganizerDecline(id) => {
                organizers::handle(id, self.user_id, OrganizerOperation::Decline, &self.pool)
                    .await?
            }
            TournamentAction::OrganizerRetract(id, invitee) => {
                organizers::handle(
                    id,
                    self.user_id,
                    OrganizerOperation::Retract(invitee),
                    &self.pool,
                )
                .await?
            }
            TournamentAction::OrganizerLeave(id) => {
                let output = organizers::handle(
                    id.clone(),
                    self.user_id,
                    OrganizerOperation::Leave,
                    &self.pool,
                )
                .await?;
                self.unsubscribe_if_no_membership(self.user_id, &id).await;
                output
            }
            TournamentAction::StartCancel(id, setup_id) => {
                StartHandler::cancel(id, setup_id, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::StartConfirm(id, setup_id, bracket_order) => {
                StartHandler::confirm(id, setup_id, bracket_order, self.user_id, &self.pool)
                    .handle()
                    .await?
                    .into()
            }
            TournamentAction::Kick(tournament_id, user) => {
                let output =
                    KickHandler::new(tournament_id.clone(), self.user_id, user, &self.pool)
                        .handle()
                        .await?;
                self.unsubscribe_if_no_membership(user, &tournament_id)
                    .await;
                output.into()
            }
            TournamentAction::Start(tournament_id, intent) => StartHandler::new(
                tournament_id,
                intent.expected_entrant_ids,
                self.user_id,
                &self.pool,
            )
            .handle()
            .await?
            .into(),
            TournamentAction::AdjudicateResult(tournament_id, intent) => {
                AdjudicateResultHandler::new(
                    tournament_id,
                    intent,
                    self.user_id,
                    &self.username,
                    self.hub.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            TournamentAction::CloseUnstarted(tournament_id, intent) => {
                CloseoutHandler::new(
                    tournament_id,
                    intent.slot_ids,
                    self.user_id,
                    &self.username,
                    self.received_from.clone(),
                    self.hub.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            TournamentAction::Withdraw(tournament_id, player) => {
                WithdrawHandler::new(
                    tournament_id,
                    player,
                    self.user_id,
                    self.hub.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            TournamentAction::ArenaJoin(tournament_id) => {
                ArenaHandler::join(tournament_id, self.user_id, self.hub.clone(), &self.pool)
                    .handle()
                    .await?
            }
            TournamentAction::ArenaPause(tournament_id) => {
                ArenaHandler::pause(tournament_id, self.user_id, self.hub.clone(), &self.pool)
                    .handle()
                    .await?
            }
            TournamentAction::ArenaResume(tournament_id) => {
                ArenaHandler::resume(tournament_id, self.user_id, self.hub.clone(), &self.pool)
                    .handle()
                    .await?
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
            | TournamentAction::InvitationAccept(id)
            | TournamentAction::OrganizerAccept(id)
            | TournamentAction::OrganizerLeave(id) => Some(id),
            TournamentAction::Kick(id, _) => Some(id),
            // An arena admits players while it runs, so a join genuinely adds
            // to the recipient set.
            TournamentAction::ArenaJoin(id) => Some(id),
            // Withdrawing does *not*: the row stays, the results stay, and the
            // player keeps receiving the tournament's messages. Only pairing
            // stops.
            TournamentAction::Withdraw(_, _)
            | TournamentAction::ArenaPause(_)
            | TournamentAction::ArenaResume(_) => None,
            // These actions change invitations, games, or tournament state,
            // but not the players ∪ organizers recipient set.
            TournamentAction::AdjudicateResult(_, _)
            | TournamentAction::CloseUnstarted(_, _)
            | TournamentAction::StartCancel(_, _)
            | TournamentAction::StartConfirm(_, _, _)
            | TournamentAction::OrganizerInvite(_, _)
            | TournamentAction::OrganizerRetract(_, _)
            | TournamentAction::OrganizerDecline(_)
            | TournamentAction::Start(_, _)
            | TournamentAction::Create(_)
            | TournamentAction::InvitationCreate(_, _)
            | TournamentAction::InvitationDecline(_)
            | TournamentAction::InvitationRetract(_, _) => None,
        };
        if let Some(id) = invalidate_id {
            self.hub.invalidate_tournament_members(id);
        }
        Ok(output)
    }
}
