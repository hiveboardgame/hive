use leptos::*;
use std::{collections::HashMap, fmt};

#[derive(Clone)]
pub enum NotificationType {
    GameInvite(String),
    TournamentInvite(String),
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            NotificationType::GameInvite(nanoid) => format!("game: {nanoid}"),
            NotificationType::TournamentInvite(nanoid) => format!("tournament: {nanoid}"),
        };
        write!(f, "Invited to {message}")
    }
}

impl NotificationType {
    pub fn get_nanoid(&self) -> String {
        match self {
            NotificationType::GameInvite(nanoid) => nanoid.clone(),
            NotificationType::TournamentInvite(nanoid) => nanoid.clone(),
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

    pub fn remove(&mut self, nanoid: &str) {
        self.notifications.update(|s| {
            s.remove(nanoid);
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
