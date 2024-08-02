use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GamesQueryOptions {
    pub username: Option<String>,
    pub last_id: Option<Uuid>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub is_finished: Option<bool>,
    pub ctx_to_update: GamesContextToUpdate,
    pub batch_size: Option<usize>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamesContextToUpdate {
    ProfileFinished,
    ProfilePlaying,
}
