use crate::responses::user::UserResponse;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hive_lib::{Bug, GameControl, GameResult, GameStatus, GameType, History, Position, State};
use serde::{Deserialize, Serialize};
use shared_types::{Conclusion, GameId, GameSpeed, GameStart, TimeMode, TournamentGameResult};
use std::cmp::Ordering;
use std::{collections::HashMap, time::Duration};
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

use cfg_if::cfg_if;

use super::tournament::TournamentAbstractResponse;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::Game,
    DbConn,
};
use hive_lib::{
    Color, GameStatus::Finished, Piece,
};
use shared_types::GamesQueryOptions;
use std::{str::FromStr, collections::HashSet};

impl GameResponse {
    pub async fn new_from_uuid(game_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        let game = Game::find_by_uuid(&game_id, conn).await?;
        GameResponse::from_model(&game, conn).await
    }

    pub async fn new_from_game_id(game_id: &GameId, conn: &mut DbConn<'_>) -> Result<Self> {
        let game = Game::find_by_game_id(game_id, conn).await?;
        GameResponse::from_model(&game, conn).await
    }

    pub async fn from_model(game: &Game, conn: &mut DbConn<'_>) -> Result<Self> {
        let history = Box::new(History::new_from_str(&game.history)?);
        let state = Box::new(State::new_from_history(&history)?);
        GameResponse::new_from(game, state, conn).await
    }

    pub async fn vec_from_options(options: GamesQueryOptions, conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let games = Game::get_rows_from_options(&options, conn).await?;
        let mut vec = Vec::new();
        for game in games {
            vec.push(GameResponse::from_model(&game, conn).await?);
        }
        Ok(vec)
    }

    pub async fn from_game_ids(game_ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let games = Game::find_by_game_ids(game_ids, conn).await?;
        let mut user_ids = HashSet::new();
        let mut tournament_ids = HashSet::new();

        for game in &games {
            user_ids.insert(game.white_id);
            user_ids.insert(game.black_id);
            if let Some(tournament_id) = game.tournament_id {
                tournament_ids.insert(tournament_id);
            }
        }

        let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();
        let tournament_ids_vec: Vec<Uuid> = tournament_ids.into_iter().collect();

        let users_map = UserResponse::from_uuids(&user_ids_vec, conn).await?;
        let tournaments_map = if !tournament_ids_vec.is_empty() {
            TournamentAbstractResponse::from_uuids(&tournament_ids_vec, conn).await?
        } else {
            HashMap::new()
        };

        let mut result = Vec::new();
        for game in games {
            let white_player = users_map.get(&game.white_id).ok_or_else(|| {
                anyhow::anyhow!("White player not found for game {}", game.id)
            })?;
            let black_player = users_map.get(&game.black_id).ok_or_else(|| {
                anyhow::anyhow!("Black player not found for game {}", game.id)
            })?;

            let tournament = game.tournament_id.and_then(|tid| tournaments_map.get(&tid));

            let history = Box::new(History::new_from_str(&game.history)?);
            let state = Box::new(State::new_from_history(&history)?);

            result.push(Self::new_from_batch(&game, state, white_player.clone(), black_player.clone(), tournament.cloned()).await?);
        }

        Ok(result)
    }

    async fn new_from(
        game: &Game,
        state: Box<State>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self> {
        let white_player = UserResponse::from_uuid(&game.white_id, conn).await?;
        let black_player = UserResponse::from_uuid(&game.black_id, conn).await?;
        let tournament = if let Some(tournament_id) = game.tournament_id {
            Some(TournamentAbstractResponse::from_uuid(&tournament_id, conn).await?)
        } else {
            None
        };

        Self::new_from_batch(game, state, white_player, black_player, tournament).await
    }

    async fn new_from_batch(
        game: &Game,
        state: Box<State>,
        white_player: UserResponse,
        black_player: UserResponse,
        tournament: Option<TournamentAbstractResponse>,
    ) -> Result<Self> {
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
            uuid: game.id,
            game_id: GameId(game.nanoid.clone()),
            tournament,
            game_status: GameStatus::from_str(&game.game_status)?,
            current_player_id: game.current_player_id,
            finished: game.finished,
            game_type: GameType::from_str(&game.game_type)?,
            tournament_queen_rule: game.tournament_queen_rule,
            turn: state.turn,
            hashes: game.hashes(),
            white_player,
            black_player,
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
            time_mode: TimeMode::from_str(&game.time_mode)?,
            time_base: game.time_base,
            time_increment: game.time_increment,
            last_interaction: game.last_interaction,
            speed: GameSpeed::from_str(&game.speed)?,
            created_at: game.created_at,
            updated_at: game.updated_at,
            conclusion: Conclusion::from_str(&game.conclusion)?,
            repetitions: state.repeating_moves.clone(),
            game_start: GameStart::from_str(&game.game_start)?,
            game_speed: GameSpeed::from_base_increment(game.time_base, game.time_increment),
            move_times: game.move_times.clone(),
            tournament_game_result: TournamentGameResult::from_str(&game.tournament_game_result)?,
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
