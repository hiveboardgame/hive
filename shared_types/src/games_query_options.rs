use crate::GameSpeed;
use chrono::{DateTime, Utc};
use hive_lib::Color;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum ResultType {
    Win,
    Loss,
    Draw,
}
impl std::fmt::Display for ResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultType::Win => write!(f, "Win"),
            ResultType::Loss => write!(f, "Loss"),
            ResultType::Draw => write!(f, "Draw"),
        }
    }
}

impl std::str::FromStr for ResultType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Win" => Ok(ResultType::Win),
            "Loss" => Ok(ResultType::Loss),
            "Draw" => Ok(ResultType::Draw),
            _ => Err(anyhow::anyhow!("Invalid ResultType string")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub struct BatchInfo {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GamesQueryOptions {
    pub players: Vec<(String, Option<Color>, Option<ResultType>)>,
    pub finished: Option<bool>,
    pub speeds: Vec<GameSpeed>,
    pub ctx_to_update: GamesContextToUpdate,
    pub current_batch: Option<BatchInfo>,
    pub batch_size: Option<usize>,
    pub unstarted: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamesContextToUpdate {
    Profile,
}
