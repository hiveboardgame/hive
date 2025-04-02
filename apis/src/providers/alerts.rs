use leptos::prelude::*;
use std::fmt;

#[derive(Clone)]
pub enum AlertType {
    Notification(String),
    Warn(String),
    Error(String),
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            AlertType::Notification(msg) => msg,
            AlertType::Warn(msg) => msg,
            AlertType::Error(msg) => msg,
        };
        write!(f, "{message}")
    }
}

#[derive(Clone)]
pub struct AlertsContext {
    pub last_alert: RwSignal<Option<AlertType>>,
}

pub fn provide_alerts() {
    provide_context(AlertsContext {
        last_alert: RwSignal::new(None),
    })
}
