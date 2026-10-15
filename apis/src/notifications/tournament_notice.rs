use crate::{common::format_utc_datetime, i18n::*};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum TournamentNotice {
    StartDeferred,
    GamesReady(usize),
    ResultsRecorded(Vec<RecordedOutcome>),
    ScheduleCancelled { actor: String, time: DateTime<Utc> },
}

#[derive(Debug, Clone, Copy)]
pub enum RecordedOutcome {
    DoubleForfeit,
    Draw,
    Win,
    Loss,
}

impl RecordedOutcome {
    fn label(self, locale: Locale) -> String {
        match self {
            Self::DoubleForfeit => td_string!(
                locale,
                notifications.tournament_notice.outcome.double_forfeit
            )
            .to_string(),
            Self::Draw => {
                td_string!(locale, notifications.tournament_notice.outcome.draw).to_string()
            }
            Self::Win => {
                td_string!(locale, notifications.tournament_notice.outcome.win).to_string()
            }
            Self::Loss => {
                td_string!(locale, notifications.tournament_notice.outcome.loss).to_string()
            }
        }
    }
}

impl TournamentNotice {
    pub fn description(&self, locale: Locale) -> String {
        match self {
            Self::StartDeferred => {
                td_string!(locale, notifications.tournament_notice.start_deferred).to_string()
            }
            Self::GamesReady(count) => td_string!(
                locale,
                notifications.tournament_notice.games_ready,
                count = count
            )
            .to_string(),
            Self::ResultsRecorded(outcomes) => {
                let labels = outcomes
                    .iter()
                    .map(|outcome| outcome.label(locale))
                    .collect::<Vec<_>>();
                if let [outcome] = labels.as_slice() {
                    td_string!(
                        locale,
                        notifications.tournament_notice.recorded_one,
                        outcome = outcome
                    )
                    .to_string()
                } else {
                    td_string!(
                        locale,
                        notifications.tournament_notice.recorded_many,
                        count = labels.len(),
                        outcomes = labels.join(", ")
                    )
                    .to_string()
                }
            }
            Self::ScheduleCancelled { actor, time } => td_string!(
                locale,
                notifications.tournament_notice.schedule_cancelled,
                actor = actor,
                time = format_utc_datetime(locale, *time)
            )
            .to_string(),
        }
    }
}
