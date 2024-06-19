use serde::{Deserialize, Serialize};
use shared_types::{TournamentDetails, TournamentId};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentAction {
    Create(Box<TournamentDetails>),
    Delete(TournamentId),
    // nanoid, user_id
    AddToSeries(TournamentId),
    Get(TournamentId),
    GetAll,
    InvitationAccept(TournamentId),
    InvitationCreate(TournamentId, Uuid),
    InvitationDecline(TournamentId),
    InvitationRetract(TournamentId, Uuid),
    Join(TournamentId),
    Leave(TournamentId),
    RemoveFromSeries(TournamentId),
    Start(TournamentId),
}
