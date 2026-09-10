use crate::tournament_configuration::Config;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TournamentStatus {
    NotStarted,
    InProgress,
    Finished,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TournamentDetails {
    pub name: String,
    pub description: Option<String>,
    pub seats: Option<i32>,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub configuration: Config,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tournament::{
        round_robin::Config as RoundRobinConfig,
        BotAdmission,
        Clock,
        FormatConfig,
        RealtimeClock,
    };
    use serde_json::json;
    use std::num::NonZeroU32;

    fn details() -> TournamentDetails {
        TournamentDetails {
            name: String::from("Forward compatible"),
            description: Some(String::from(
                "A sufficiently long tournament description for testing.",
            )),
            seats: Some(8),
            min_seats: 4,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
                    NonZeroU32::new(2).unwrap(),
                    Clock::Realtime(RealtimeClock {
                        base_seconds: NonZeroU32::new(300).unwrap(),
                        increment_seconds: 3,
                    }),
                )),
            },
        }
    }

    #[test]
    fn details_are_forward_compatible_but_configuration_is_strict() {
        let mut value = serde_json::to_value(details()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert(String::from("future_ui_hint"), json!(true));
        assert!(serde_json::from_value::<TournamentDetails>(value.clone()).is_ok());

        let mut value = serde_json::to_value(details()).unwrap();
        value["configuration"]
            .as_object_mut()
            .unwrap()
            .insert(String::from("future_authoritative_field"), json!(true));
        assert!(serde_json::from_value::<TournamentDetails>(value).is_err());
    }
}
