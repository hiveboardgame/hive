use serde::{Deserialize, Serialize};
use shared_types::{tournament::Resolution, TournamentDetails, TournamentGameResult, TournamentId};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TournamentAdjudicationIntent {
    pub slot_id: Uuid,
    pub expected_resolution: Option<Resolution>,
    pub result: TournamentGameResult,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TournamentCloseoutIntent {
    pub slot_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TournamentStartIntent {
    pub expected_entrant_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TournamentAction {
    // TODO: AddToSeries(TournamentId),
    AdjudicateResult(TournamentId, TournamentAdjudicationIntent),
    CloseUnstarted(TournamentId, TournamentCloseoutIntent),
    Create(Box<TournamentDetails>),
    Delete(TournamentId),
    InvitationAccept(TournamentId),
    InvitationCreate(TournamentId, Uuid),
    InvitationDecline(TournamentId),
    InvitationRetract(TournamentId, Uuid),
    OrganizerInvite(TournamentId, Uuid),
    OrganizerAccept(TournamentId),
    OrganizerDecline(TournamentId),
    OrganizerRetract(TournamentId, Uuid),
    OrganizerLeave(TournamentId),
    Join(TournamentId),
    Kick(TournamentId, Uuid),
    Leave(TournamentId),
    // TODO: RemoveFromSeries(TournamentId),
    Start(TournamentId, TournamentStartIntent),
    StartCancel(TournamentId, Uuid),
    StartConfirm(TournamentId, Uuid, Vec<Uuid>),
    Withdraw(TournamentId, Uuid),
    ArenaJoin(TournamentId),
    ArenaPause(TournamentId),
    ArenaResume(TournamentId),
}
