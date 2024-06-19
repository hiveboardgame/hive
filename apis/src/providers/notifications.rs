use leptos::*;
use shared_types::ChallengeId;
use std::{collections::HashMap, fmt};

#[derive(Clone, Hash)]
pub enum NotificationType {
    GameInvite(ChallengeId),
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            NotificationType::GameInvite(nanoid) => format!("game: {nanoid}"),
        };
        write!(f, "Invited to {message}")
    }
}

impl NotificationType {
    pub fn get_nanoid(&self) -> String {
        match self {
            NotificationType::GameInvite(game_id) => game_id.0.clone(),
        }
    }
}

#[derive(Clone)]
pub struct NotificationContext {
    pub notifications: RwSignal<HashMap<String, NotificationType>>,
}

impl NotificationContext {
    pub fn new() -> Self {
        Self {
            notifications: RwSignal::new(HashMap::new()),
        }
    }

    pub fn remove(&mut self, challenge_id: &ChallengeId) {
        self.notifications.update(|s| {
            s.remove(&challenge_id.0);
        });
    }

    pub fn add(&mut self, notifications: Vec<NotificationType>) {
        self.notifications.update(|s| {
            for notification in notifications {
                s.insert(notification.get_nanoid(), notification);
            }
        })
    }
}

impl Default for NotificationContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_notifications() {
    provide_context(NotificationContext::default())
}
