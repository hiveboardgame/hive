use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::GameResponse;
use shared_types::GamesContextToUpdate;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GamesSearchResponse {
    pub results: Vec<GameResponse>,
    pub ctx_to_update: GamesContextToUpdate,
    pub last_id: Option<Uuid>,
    pub last_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub more_rows: bool,
}
