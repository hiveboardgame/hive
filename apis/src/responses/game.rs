use crate::responses::user::UserResponse;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hive_lib::{
    bug::Bug, game_control::GameControl, game_result::GameResult, game_status::GameStatus,
    game_type::GameType, history::History, position::Position, state::State,
};
use serde::{Deserialize, Serialize};
use shared_types::{conclusion::Conclusion, game_speed::GameSpeed, time_mode::TimeMode};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GameResponse {
    pub game_id: Uuid,
    pub nanoid: String,
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
}

impl GameResponse {
    pub fn white_rating(&self) -> u64 {
        self.white_player.rating_for_speed(&self.speed)
    }

    pub fn black_rating(&self) -> u64 {
        self.black_player.rating_for_speed(&self.speed)
    }

    pub fn create_state(&self) -> State {
        let result = match &self.game_status {
            &GameStatus::NotStarted | &GameStatus::InProgress => GameResult::Unknown,
            GameStatus::Finished(result) => result.clone(),
        };
        State::new_from_history(&History::new_from_gamestate(
            self.history.clone(),
            &self.hashes,
            result,
            self.game_type,
        ))
        .expect("State to be valid, as game was")
    }

    pub fn time_left(&self) -> Result<std::time::Duration> {
        if self.turn < 2 {
            return Ok(std::time::Duration::from_nanos(std::u64::MAX));
        }
        if self.time_mode == TimeMode::Untimed {
            return Ok(self
                .updated_at
                .signed_duration_since(DateTime::<chrono::Utc>::MIN_UTC)
                .to_std()?);
        }
        if let Some(interaction) = self.last_interaction {
            let left = if self.turn % 2 == 0 {
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
        Ok(std::time::Duration::from_nanos(std::u64::MAX))
    }
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::game::Game,
    DbPool,
};
use hive_lib::{
    color::Color, game_status::GameStatus::Finished, piece::Piece,
};
use std::str::FromStr;

impl GameResponse {
    pub async fn new_from_uuid(game_id: Uuid, pool: &DbPool) -> Result<Self> {
        let game = Game::find_by_uuid(&game_id, pool).await?;
        GameResponse::new_from_db(&game, pool).await
    }

    pub async fn new_from_nanoid(game_id: &str, pool: &DbPool) -> Result<Self> {
        let game = Game::find_by_nanoid(game_id, pool).await?;
        GameResponse::new_from_db(&game, pool).await
    }

    pub async fn new_from_db(game: &Game, pool: &DbPool) -> Result<Self> {
        let history = History::new_from_str(&game.history)?;
        let state = State::new_from_history(&history)?;
        GameResponse::new_from(game, &state, pool).await
    }

    async fn new_from(
        game: &Game,
        state: &State,
        pool: &DbPool,
    ) -> Result<Self> {
        let white_player = UserResponse::from_uuid(&game.white_id, pool).await?;
        let black_player = UserResponse::from_uuid(&game.black_id, pool).await?;
        let (white_rating, black_rating, white_rating_change, black_rating_change) = {
            if let Finished(_) = GameStatus::from_str(&game.game_status).expect("GameStatus parsed") {
                (
                    game.white_rating,
                    game.black_rating,
                    game.white_rating_change,
                    game.black_rating_change,
                )
            } else {
                (
                    Some(white_player.rating_for_speed(&GameSpeed::from_str(&game.speed)?) as f64),
                    Some(black_player.rating_for_speed(&GameSpeed::from_str(&game.speed)?) as f64),
                    None,
                    None,
                )
            }
        };
        let white_time_left = game.white_time_left.map(|nanos| Duration::from_nanos(nanos as u64));
        let black_time_left = game.black_time_left.map(|nanos| Duration::from_nanos(nanos as u64));
        Ok(Self {
            game_id: game.id,
            nanoid: game.nanoid.clone(),
            game_status: GameStatus::from_str(&game.game_status)?,
            current_player_id: game.current_player_id,
            finished: game.finished,
            game_type: GameType::from_str(&game.game_type)?,
            tournament_queen_rule: game.tournament_queen_rule,
            turn: state.turn,
            hashes: game.hashes(),
            white_player: white_player,
            black_player: black_player,
            moves: GameResponse::moves_as_string(state.board.moves(state.turn_color)),
            spawns: state
                .board
                .spawnable_positions(state.turn_color)
                .collect::<Vec<_>>(),
            rated: game.rated,
            reserve_black: state
                .board
                .reserve(Color::Black, game.game_type.parse().expect("Gametype parsed")),
            reserve_white: state
                .board
                .reserve(Color::White, game.game_type.parse().expect("Gametype parsed")),
            history: state.history.moves.clone(),
            game_control_history: Self::gc_history(&game.game_control_history),
            white_rating,
            black_rating,
            white_rating_change,
            black_rating_change,
            white_time_left,
            black_time_left,
            time_mode: TimeMode::from_str(&game.time_mode).unwrap(),
            time_base: game.time_base,
            time_increment: game.time_increment,
            last_interaction: game.last_interaction,
            speed: GameSpeed::from_str(&game.speed)?,
            created_at: game.created_at,
            updated_at: game.updated_at,
            conclusion: Conclusion::from_str(&game.conclusion)?,
            repetitions: state.repeating_moves.clone(),
        })
    }

    fn gc_history(gcs: &str) -> Vec<(i32, GameControl)> {
        let mut ret = Vec::new();
        for gc_str in gcs.split_terminator(';') {
            let turn: i32;
            let gc: GameControl;
            // TODO: This code is janky
            if let Some(turn_str) = gc_str.split(' ').next() {
                turn = turn_str.strip_suffix('.').expect("Suffix exists").parse().expect("Turn parsed");
                if let Some(gc_token) = gc_str.split(' ').nth(1) {
                    gc = gc_token.parse().expect("Token parsed");
                    ret.push((turn, gc));
                }
            }
        }
        ret
    }

    fn moves_as_string(
        moves: HashMap<(Piece, Position), Vec<Position>>,
    ) -> HashMap<String, Vec<Position>> {
        let mut mapped = HashMap::new();
        for ((piece, _pos), possible_pos) in moves.into_iter() {
            mapped.insert(piece.to_string(), possible_pos);
        }
        mapped
    }
}
}}
