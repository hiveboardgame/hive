use crate::{common::localized_error_message, i18n::*};
use leptos::prelude::*;
use std::fmt;

#[derive(Clone)]
pub enum AlertType {
    Notification(String),
    Warn(String),
    Error(String),
    GameAborted(String),
    TournamentSlotsClosed(u32),
    TournamentStartDeferred,
    TournamentStartNeedsEntrants(i32),
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            Self::GameAborted(actor) => {
                return write!(
                    f,
                    "{}",
                    t_string!(use_i18n(), game.aborted_by, actor = actor.clone())
                )
            }
            Self::TournamentSlotsClosed(1) => {
                return write!(
                    f,
                    "{}",
                    t_string!(use_i18n(), tournaments.alerts.slots_closed_one)
                )
            }
            Self::TournamentSlotsClosed(count) => {
                return write!(
                    f,
                    "{}",
                    t_string!(
                        use_i18n(),
                        tournaments.alerts.slots_closed_many,
                        count = *count
                    )
                )
            }
            Self::TournamentStartDeferred => {
                return write!(
                    f,
                    "{}",
                    t_string!(use_i18n(), tournaments.alerts.start_deferred)
                )
            }
            Self::TournamentStartNeedsEntrants(minimum) => {
                return write!(
                    f,
                    "{}",
                    t_string!(
                        use_i18n(),
                        tournaments.alerts.start_needs_entrants,
                        minimum = *minimum
                    )
                )
            }
            AlertType::Notification(msg) => msg,
            AlertType::Warn(msg) => msg,
            AlertType::Error(msg) => {
                return write!(f, "{}", localized_error_message(use_i18n(), msg))
            }
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
