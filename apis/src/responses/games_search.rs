use super::GameResponse;
use serde::{Deserialize, Serialize};
use shared_types::{BatchInfo, GamesContextToUpdate};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GamesSearchResponse {
    pub results: Vec<GameResponse>,
    pub ctx_to_update: GamesContextToUpdate,
    pub batch: Option<BatchInfo>,
    pub more_rows: bool,
    pub first_batch: bool,
}
