use crate::PrettyString;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentSortOrder {
    CreatedAtDesc,
    CreatedAtAsc,
    StartedAtDesc,
    StartedAtAsc,
}
