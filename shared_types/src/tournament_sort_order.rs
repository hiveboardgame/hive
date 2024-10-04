use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentSortOrder {
    CreatedAtDesc,
    CreatedAtAsc,
    StartedAtDesc,
    StartedAtAsc,
}
