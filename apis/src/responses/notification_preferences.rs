use serde::{Deserialize, Serialize};
use shared_types::NotificationCategory;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NotificationPreferencesResponse {
    pub your_turn: Vec<String>,
    pub challenges: Vec<String>,
    pub game_ended: Vec<String>,
    pub tournament: Vec<String>,
    pub schedules: Vec<String>,
    pub dms: Vec<String>,
}

impl NotificationPreferencesResponse {
    pub fn channels(&self, category: NotificationCategory) -> &Vec<String> {
        match category {
            NotificationCategory::YourTurn => &self.your_turn,
            NotificationCategory::Challenges => &self.challenges,
            NotificationCategory::GameEnded => &self.game_ended,
            NotificationCategory::Tournament => &self.tournament,
            NotificationCategory::Schedules => &self.schedules,
            NotificationCategory::Dms => &self.dms,
        }
    }

    pub fn channels_mut(&mut self, category: NotificationCategory) -> &mut Vec<String> {
        match category {
            NotificationCategory::YourTurn => &mut self.your_turn,
            NotificationCategory::Challenges => &mut self.challenges,
            NotificationCategory::GameEnded => &mut self.game_ended,
            NotificationCategory::Tournament => &mut self.tournament,
            NotificationCategory::Schedules => &mut self.schedules,
            NotificationCategory::Dms => &mut self.dms,
        }
    }
}

#[cfg(feature = "ssr")]
impl From<db_lib::models::NotificationPreferences> for NotificationPreferencesResponse {
    fn from(p: db_lib::models::NotificationPreferences) -> Self {
        Self {
            your_turn: flatten(p.your_turn),
            challenges: flatten(p.challenges),
            game_ended: flatten(p.game_ended),
            tournament: flatten(p.tournament),
            schedules: flatten(p.schedules),
            dms: flatten(p.dms),
        }
    }
}

#[cfg(feature = "ssr")]
fn flatten(v: Vec<Option<String>>) -> Vec<String> {
    v.into_iter().flatten().collect()
}
