use std::{fmt::Display, str::FromStr};

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
impl Display for ResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultType::Win => write!(f, "Win"),
            ResultType::Loss => write!(f, "Loss"),
            ResultType::Draw => write!(f, "Draw"),
        }
    }
}

impl FromStr for ResultType {
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

#[derive(Clone, PartialEq, Copy, Debug, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameProgress {
    Unstarted,
    #[default]
    Playing,
    Finished,
    All,
}
impl FromStr for GameProgress {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unstarted" => Ok(GameProgress::Unstarted),
            "Playing" => Ok(GameProgress::Playing),
            "Finished" => Ok(GameProgress::Finished),
            "All" => Ok(GameProgress::All),
            _ => Err(anyhow::anyhow!("Invalid GameProgress string")),
        }
    }
}
impl Display for GameProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view = match self {
            GameProgress::Unstarted => "Unstarted",
            GameProgress::Playing => "Playing",
            GameProgress::Finished => "Finished",
            GameProgress::All => "All",
        };
        write!(f, "{view}")
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
    pub speeds: Vec<GameSpeed>,
    pub ctx_to_update: GamesContextToUpdate,
    pub current_batch: Option<BatchInfo>,
    pub batch_size: Option<usize>,
    pub game_progress: GameProgress,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamesContextToUpdate {
    Profile,
}
