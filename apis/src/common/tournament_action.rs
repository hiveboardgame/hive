use serde::{Deserialize, Serialize};
use shared_types::TournamentDetails;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentAction {
    Create(Box<TournamentDetails>),
    Delete(String),
    // nanoid, user_id
    AddToSeries(String),
    Get(String),
    GetAll,
    InvitationAccept(String),
    InvitationCreate(String, Uuid),
    InvitationDecline(String),
    InvitationRetract(String, Uuid),
    Join(String),
    Leave(String),
    RemoveFromSeries(String),
    Start(String),
}
