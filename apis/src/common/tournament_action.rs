use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentDetails, TournamentGameResult, TournamentId};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentResponseDepth {
    Full,
    Abstract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentAction {
    Abandon(TournamentId),
    // TODO: AddToSeries(TournamentId),
    AdjudicateResult(GameId, TournamentGameResult),
    Create(Box<TournamentDetails>),
    Delete(TournamentId),
    Get(TournamentId, TournamentResponseDepth),
    GetAll(TournamentResponseDepth),
    InvitationAccept(TournamentId),
    InvitationCreate(TournamentId, Uuid),
    InvitationDecline(TournamentId),
    InvitationRetract(TournamentId, Uuid),
    Join(TournamentId),
    Kick(TournamentId, Uuid),
    Leave(TournamentId),
    // TODO: RemoveFromSeries(TournamentId),
    Start(TournamentId),
}
