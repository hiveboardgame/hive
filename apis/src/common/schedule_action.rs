use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleAction {
    Propose(DateTime<Utc>, GameId),
    Accept(Uuid),
    Cancel(Uuid),
    TournamentPublic(TournamentId),
    TournamentOwn(TournamentId),
}

impl fmt::Display for ScheduleAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Propose(date, game_id) => write!(f, "Propose({date}, {game_id})"),
            Self::Accept(game_id) => write!(f, "Accept({game_id})"),
            Self::Cancel(game_id) => write!(f, "Cancel({game_id})"),
            Self::TournamentPublic(id) => write!(f, "TournamentPublic({id})"),
            Self::TournamentOwn(id) => write!(f, "TournamentOwn({id})"),
        }
    }
}
