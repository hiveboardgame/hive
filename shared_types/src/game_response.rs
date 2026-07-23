use crate::{
    BatchToken,
    Conclusion,
    GameId,
    GameSpeed,
    GameStart,
    TimeMode,
    TournamentAbstractResponse,
    TournamentGameResult,
    UserResponse,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hive_lib::{Bug, GameControl, GameResult, GameStatus, GameType, History, Position, State};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap, time::Duration};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct GameAbstractResponse {
    pub tournament: Option<TournamentAbstractResponse>,
    pub game_id: GameId,
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_rating_change: Option<f64>,
    pub black_rating_change: Option<f64>,
    pub history: Vec<(String, String)>,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub speed: GameSpeed,
    pub conclusion: Conclusion,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GameResponse {
    pub uuid: Uuid,
    pub game_id: GameId,
    pub tournament: Option<TournamentAbstractResponse>,
    pub current_player_id: Uuid,
    pub turn: usize,
    pub finished: bool,
    pub game_status: GameStatus,
    pub game_type: GameType,
    pub tournament_queen_rule: bool,
    pub white_player: UserResponse,
    pub black_player: UserResponse,
    pub moves: HashMap<String, Vec<Position>>,
    pub spawns: Vec<Position>,
    pub rated: bool,
    pub reserve_black: HashMap<Bug, Vec<String>>,
    pub reserve_white: HashMap<Bug, Vec<String>>,
    pub history: Vec<(String, String)>,
    pub game_control_history: Vec<(i32, GameControl)>,
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_rating_change: Option<f64>,
    pub black_rating_change: Option<f64>,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub speed: GameSpeed,
    pub black_time_left: Option<Duration>,
    pub white_time_left: Option<Duration>,
    pub last_interaction: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hashes: Vec<u64>,
    pub conclusion: Conclusion,
    pub repetitions: Vec<usize>,
    pub game_start: GameStart,
    pub game_speed: GameSpeed,
    pub move_times: Vec<Option<i64>>,
    pub tournament_game_result: TournamentGameResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameBatchResponse {
    pub games: Vec<GameResponse>,
    pub next_batch: Option<BatchToken>,
    pub total: Option<i64>,
}

impl PartialEq for GameResponse {
    fn eq(&self, other: &Self) -> bool {
        self.game_id == other.game_id
            && self.turn == other.turn
            && self.finished == other.finished
            && self.last_interaction == other.last_interaction
    }
}

impl Ord for GameResponse {
    fn cmp(&self, other: &Self) -> Ordering {
        self.game_id.0.cmp(&other.game_id.0)
    }
}

impl PartialOrd for GameResponse {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for GameResponse {}

impl GameResponse {
    pub fn recorded_time_left(&self, turn: usize) -> Option<Duration> {
        self.move_times
            .get(turn)
            .copied()
            .flatten()
            .and_then(|nanos| u64::try_from(nanos).ok())
            .map(Duration::from_nanos)
    }

    pub fn white_rating(&self) -> u64 {
        self.white_player.rating_for_speed(&self.speed)
    }

    pub fn black_rating(&self) -> u64 {
        self.black_player.rating_for_speed(&self.speed)
    }

    pub fn create_state(&self) -> State {
        let result = match &self.game_status {
            &GameStatus::NotStarted | &GameStatus::InProgress | &GameStatus::Adjudicated => {
                GameResult::Unknown
            }
            GameStatus::Finished(result) => result.clone(),
        };
        let mut state = State::new_from_history(&History::new_from_gamestate(
            self.history.clone(),
            &self.hashes,
            result,
            self.game_type,
        ))
        .expect("State to be valid, as game was");
        state.game_status = self.game_status.clone();
        state.tournament = self.tournament_queen_rule;
        state
    }

    /// Preview URLs are user-controlled, so clamp before replaying history.
    pub fn create_state_at_turn(&self, turn: usize) -> State {
        let turn = turn.min(self.history.len());
        State::new_from_history(&History::new_from_gamestate(
            self.history[..turn].to_vec(),
            &self.hashes[..turn.min(self.hashes.len())],
            GameResult::Unknown,
            self.game_type,
        ))
        .expect("Partial state to be valid, as the full game was")
    }

    pub fn organizer_can_adjudicate(&self) -> bool {
        matches!(
            self.conclusion,
            Conclusion::Unknown | Conclusion::Committee | Conclusion::Forfeit
        ) && self.turn == 0
            && self.history.is_empty()
            && self.game_start == GameStart::Ready
            && matches!(
                self.game_status,
                GameStatus::NotStarted | GameStatus::Adjudicated
            )
    }

    pub fn time_left(&self) -> Result<std::time::Duration> {
        if self.turn < 2 {
            return Ok(std::time::Duration::from_nanos(u64::MAX));
        }
        if self.time_mode == TimeMode::Untimed {
            return Ok(self
                .updated_at
                .signed_duration_since(DateTime::<chrono::Utc>::MIN_UTC)
                .to_std()?);
        }
        if let Some(interaction) = self.last_interaction {
            let left = if self.turn.is_multiple_of(2) {
                chrono::Duration::from_std(
                    self.white_time_left.context("white_time_left not some")?,
                )
            } else {
                chrono::Duration::from_std(
                    self.black_time_left.context("black_time_left not some")?,
                )
            }
            .context("Could not convert to chrono::TimeDelta")?;
            let future = interaction
                .checked_add_signed(left)
                .context("Time overflowed")?;
            let now = Utc::now();
            if now > future {
                return Ok(std::time::Duration::from_nanos(0));
            } else {
                return Ok(future.signed_duration_since(now).to_std()?);
            }
        }
        Ok(std::time::Duration::from_nanos(u64::MAX))
    }
}
