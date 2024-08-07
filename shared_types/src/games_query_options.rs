use crate::GameSpeed;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BatchInfo {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GamesQueryOptions {
    pub usernames: Vec<String>,
    pub is_finished: Option<bool>,
    pub speeds: Vec<GameSpeed>,
    pub ctx_to_update: GamesContextToUpdate,
    pub current_batch: Option<BatchInfo>,
    pub batch_size: Option<usize>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamesContextToUpdate {
    ProfileFinished,
    ProfilePlaying,
}
