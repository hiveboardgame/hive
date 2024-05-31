use serde::{Deserialize, Serialize};
use shared_types::TournamentDetails;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentAction {
    Create(Box<TournamentDetails>),
    Delete(String),
    AcceptInvitation(String),
    DeclineInvitation(String),
    AddToSeries(String),
    RemoveFromSeries(String),
    Join(String),
    Leave(String),
    Invite(String),
    Start(String),
    Get(String),
    GetAll,
}
