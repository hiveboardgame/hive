use std::{collections::HashMap, sync::RwLock};
use chrono::{DateTime, Utc};
use shared_types::GameId;
use uuid::Uuid;

#[derive(Debug)]
pub struct TournamentGameStart {
    pub tournament: RwLock<HashMap<GameId, (Uuid, DateTime<Utc>)>>,
}

impl TournamentGameStart {
    pub fn new() -> Self {
        Self { tournament: RwLock::new(HashMap::new()) }
    }
}

impl Default for TournamentGameStart {
    fn default() -> Self {
        Self::new()
    }
}


