use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::TournamentId;
use std::fmt::{Display, Formatter, Result as FmtResult};
use uuid::Uuid;

const SCHEDULE_SLOT_FRAGMENT_PREFIX: &str = "schedule-slot-";

pub fn schedule_slot_fragment(slot_id: Uuid) -> String {
    format!("{SCHEDULE_SLOT_FRAGMENT_PREFIX}{slot_id}")
}

pub fn parse_schedule_slot_fragment(fragment: &str) -> Option<Uuid> {
    fragment
        .strip_prefix('#')
        .unwrap_or(fragment)
        .strip_prefix(SCHEDULE_SLOT_FRAGMENT_PREFIX)
        .and_then(|slot_id| slot_id.parse().ok())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleAction {
    Propose {
        candidate_times: Vec<DateTime<Utc>>,
        tournament_id: TournamentId,
        slot_id: Uuid,
    },
    Accept {
        offer_id: Uuid,
        selected_time: DateTime<Utc>,
    },
    Decline(Uuid),
    Withdraw(Uuid),
    SetDeadline {
        tournament_id: TournamentId,
        slot_ids: Vec<Uuid>,
        deadline_at: Option<DateTime<Utc>>,
    },
}

impl Display for ScheduleAction {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Propose {
                candidate_times,
                tournament_id,
                slot_id,
            } => {
                write!(
                    f,
                    "Propose({candidate_times:?}, {tournament_id}, {slot_id})"
                )
            }
            Self::Accept {
                offer_id,
                selected_time,
            } => write!(f, "Accept({offer_id}, {selected_time})"),
            Self::Decline(offer_id) => write!(f, "Decline({offer_id})"),
            Self::Withdraw(offer_id) => write!(f, "Withdraw({offer_id})"),
            Self::SetDeadline {
                tournament_id,
                slot_ids,
                deadline_at,
            } => write!(
                f,
                "SetDeadline({tournament_id}, {slot_ids:?}, {deadline_at:?})"
            ),
        }
    }
}
