use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationCategory {
    YourTurn,
    Challenges,
    GameEnded,
    Tournament,
    Schedules,
    Dms,
    GeneralChat,
}

impl NotificationCategory {
    pub const ALL: [NotificationCategory; 7] = [
        Self::YourTurn,
        Self::Challenges,
        Self::GameEnded,
        Self::Tournament,
        Self::Schedules,
        Self::Dms,
        Self::GeneralChat,
    ];

    pub fn column(&self) -> &'static str {
        match self {
            Self::YourTurn => "your_turn",
            Self::Challenges => "challenges",
            Self::GameEnded => "game_ended",
            Self::Tournament => "tournament",
            Self::Schedules => "schedules",
            Self::Dms => "dms",
            Self::GeneralChat => "general_chat",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_covers_every_category() {
        assert_eq!(NotificationCategory::ALL.len(), 7);
    }

    #[test]
    fn general_chat_column_name() {
        assert_eq!(NotificationCategory::GeneralChat.column(), "general_chat");
    }
}
