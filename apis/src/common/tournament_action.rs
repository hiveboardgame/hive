use serde::{Deserialize, Serialize};
use shared_types::TournamentDetails;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentAction {
    Create(Box<TournamentDetails>),
    Delete(String),
    // nanoid, user_id
    InvitationCreate(String, Uuid),
    InvitationDecline(String),
    InvitationAccept(String),
    AddToSeries(String),
    RemoveFromSeries(String),
    Join(String),
    Leave(String),
    Start(String),
    Get(String),
    GetAll,
}
