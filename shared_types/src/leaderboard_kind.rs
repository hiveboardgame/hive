use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaderboardKind {
    Humans,
    Bots,
}

impl LeaderboardKind {
    pub fn is_bots(&self) -> bool {
        matches!(self, LeaderboardKind::Bots)
    }
}
