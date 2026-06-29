use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationCategory {
    YourTurn,
    Challenges,
    GameEnded,
    Tournament,
    Schedules,
    Dms,
}

impl NotificationCategory {
    pub const ALL: [NotificationCategory; 6] = [
        Self::YourTurn,
        Self::Challenges,
        Self::GameEnded,
        Self::Tournament,
        Self::Schedules,
        Self::Dms,
    ];

    pub fn column(&self) -> &'static str {
        match self {
            Self::YourTurn => "your_turn",
            Self::Challenges => "challenges",
            Self::GameEnded => "game_ended",
            Self::Tournament => "tournament",
            Self::Schedules => "schedules",
            Self::Dms => "dms",
        }
    }
}
