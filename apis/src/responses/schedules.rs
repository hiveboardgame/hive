use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub proposer_id: Uuid,
    pub game_id: Uuid,
    pub start_t: DateTime<Utc>,
    pub agreed: bool,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::Schedule,
};
impl ScheduleResponse {
    pub fn from_model(schedule: Schedule) -> Self {
        Self {
            id: schedule.id,
            proposer_id: schedule.proposer_id,
            game_id: schedule.game_id,
            start_t: schedule.start_t,
            agreed: schedule.agreed,
        }
    }
    pub fn ids_only(game_id: Uuid, proposer_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            proposer_id,
            game_id,
            start_t: Utc::now(),
            agreed: false,
        }
    }
    pub fn new(game_id: Uuid, proposer_id: Uuid, start_t: DateTime<Utc>, agreed: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            game_id,
            proposer_id,
            start_t,
            agreed,
        }
    }
}
}
}
