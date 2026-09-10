use crate::responses::{user::UserResponse, TournamentAbstractResponse};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use hive_lib::{Bug, GameControl, GameResult, GameStatus, GameType, History, Position, State};
use serde::{Deserialize, Serialize};
use shared_types::{
    clock::Clock,
    BatchToken,
    Conclusion,
    GameId,
    GameSpeed,
    GameStart,
    TimeMode,
    TournamentGameResult,
};
#[cfg(feature = "ssr")]
use shared_types::{GamesQueryOptions, TournamentId};
use std::{cmp::Ordering, collections::HashMap, time::Duration};
use uuid::Uuid;

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
    /// Berserk halves that player's starting clock and drops their increment
    /// entirely, so both sides' timers depend on these.
    pub white_berserked: bool,
    pub black_berserked: bool,
    pub last_interaction: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// The separate first-move deadline while an Arena game is waiting for
    /// each player's opening move. Ordinary clocks remain held while present.
    pub arena_move_due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hashes: Vec<u64>,
    pub conclusion: Conclusion,
    pub repetitions: Vec<usize>,
    pub game_start: GameStart,
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
            && self.finished_at == other.finished_at
            && self.arena_move_due_at == other.arena_move_due_at
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
    pub fn time_control(&self) -> Option<Clock> {
        Clock::from_time_parts(self.time_mode, self.time_base, self.time_increment)
            .expect("game response time control is validated when constructed")
    }

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

    pub fn time_left(&self) -> Result<Duration> {
        let clock_is_held = self.game_status == GameStatus::NotStarted
            || (self.game_start == GameStart::Arena && self.turn < 2);
        if clock_is_held {
            return Ok(Duration::from_nanos(u64::MAX));
        }
        if self.time_mode == TimeMode::Untimed {
            return Ok(self
                .updated_at
                .signed_duration_since(DateTime::<Utc>::MIN_UTC)
                .to_std()?);
        }
        if let Some(interaction) = self.last_interaction {
            let left = if self.turn.is_multiple_of(2) {
                ChronoDuration::from_std(self.white_time_left.context("white_time_left not some")?)
            } else {
                ChronoDuration::from_std(self.black_time_left.context("black_time_left not some")?)
            }
            .context("Could not convert to chrono::TimeDelta")?;
            let future = interaction
                .checked_add_signed(left)
                .context("Time overflowed")?;
            let now = Utc::now();
            if now > future {
                return Ok(Duration::from_nanos(0));
            } else {
                return Ok(future.signed_duration_since(now).to_std()?);
            }
        }
        Ok(Duration::from_nanos(u64::MAX))
    }
}

use cfg_if::cfg_if;

cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Game, Tournament},
    DbConn,
};
use hive_lib::{
    Color, GameStatus::Finished, Piece,
};
use std::{collections::HashSet, str::FromStr};

fn tournament_abstract(tournament: &Tournament) -> Result<TournamentAbstractResponse> {
    Ok(TournamentAbstractResponse {
        tournament_id: TournamentId(tournament.nanoid.clone()),
        name: tournament.name.clone(),
        players: 0,
        joined: false,
        invited: false,
        organizing: false,
        seats: tournament.seats,
        invite_only: tournament.invite_only,
        configuration: tournament.configuration().clone(),
        band_upper: tournament.band_upper,
        band_lower: tournament.band_lower,
        starts_at: tournament.starts_at,
        started_at: tournament.started_at,
        finished_at: tournament.finished_at,
    })
}

impl GameResponse {
    pub async fn new_from_game_id(game_id: &GameId, conn: &mut DbConn<'_>) -> Result<Self> {
        let game = Game::find_by_game_id(game_id, conn).await?;
        GameResponse::from_model(&game, conn).await
    }

    pub async fn from_model(game: &Game, conn: &mut DbConn<'_>) -> Result<Self> {
        let state = Box::new(State::new_from_str(&game.history, &game.game_type)?);
        GameResponse::new_from(game, state, conn).await
    }

    pub async fn batch_from_options(
        options: GamesQueryOptions,
        conn: &mut DbConn<'_>,
    ) -> Result<GameBatchResponse> {
        let (games, next_batch, total) = Game::get_rows_from_options(&options, conn).await?;
        let games = Self::from_games_batch(games, conn).await?;
        Ok(GameBatchResponse {
            games,
            next_batch,
            total,
        })
    }

    pub async fn from_game_ids(game_ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let games = Game::find_by_game_ids(game_ids, conn).await?;
        Self::from_games_batch(games, conn).await
    }

    pub async fn from_games_batch(games: Vec<Game>, conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let mut user_ids = HashSet::new();
        for game in &games {
            user_ids.insert(game.white_id);
            user_ids.insert(game.black_id);
        }

        let tournament_ids = games
            .iter()
            .filter_map(|game| game.tournament_id)
            .collect::<HashSet<_>>();
        let tournament_ids_vec = tournament_ids.iter().copied().collect::<Vec<_>>();
        let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();

        let users_map = UserResponse::from_uuids(&user_ids_vec, conn).await?;
        let mut tournaments_map = HashMap::new();
        for tournament in Tournament::find_by_uuids(&tournament_ids_vec, conn).await? {
            tournaments_map.insert(tournament.id, tournament);
        }

        let mut result = Vec::new();
        for game in games {
            let white_player = users_map.get(&game.white_id).cloned().ok_or_else(|| {
                anyhow::anyhow!("White player not found for game {}", game.id)
            })?;
            let black_player = users_map.get(&game.black_id).cloned().ok_or_else(|| {
                anyhow::anyhow!("Black player not found for game {}", game.id)
            })?;

            let tournament = game
                .tournament_id
                .map(|id| {
                    tournaments_map
                        .get(&id)
                        .ok_or_else(|| anyhow::anyhow!("Tournament {id} not found"))
                        .and_then(tournament_abstract)
                })
                .transpose()?;

            let state = Box::new(State::new_from_str(&game.history, &game.game_type)?);

            result.push(Self::new_from_batch(
                &game,
                state,
                white_player,
                black_player,
                tournament,
            )?);
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
        let tournament = match game.tournament_id {
            Some(id) => Some(tournament_abstract(&Tournament::find(id, conn).await?)?),
            None => None,
        };

        Self::new_from_batch(game, state, white_player, black_player, tournament)
    }

    fn new_from_batch(
        game: &Game,
        state: Box<State>,
        white_player: UserResponse,
        black_player: UserResponse,
        tournament: Option<TournamentAbstractResponse>,
    ) -> Result<Self> {
        let game_status = GameStatus::from_str(&game.game_status)?;
        let game_speed = GameSpeed::from_str(&game.speed)?;
        let game_type = GameType::from_str(&game.game_type)?;
        let (white_rating, black_rating, white_rating_change, black_rating_change) = {
            if matches!(&game_status, Finished(_)) {
                (
                    game.white_rating,
                    game.black_rating,
                    game.white_rating_change,
                    game.black_rating_change,
                )
            } else {
                (
                    Some(white_player.rating_for_speed(&game_speed) as f64),
                    Some(black_player.rating_for_speed(&game_speed) as f64),
                    None,
                    None,
                )
            }
        };
        let white_time_left = game.white_time_left.map(|nanos| Duration::from_nanos(nanos as u64));
        let black_time_left = game.black_time_left.map(|nanos| Duration::from_nanos(nanos as u64));
        let time_mode = TimeMode::from_str(&game.time_mode)?;
        Clock::from_time_parts(time_mode, game.time_base, game.time_increment)?;
        Ok(Self {
            uuid: game.id,
            game_id: GameId(game.nanoid.clone()),
            tournament,
            game_status,
            current_player_id: game.current_player_id,
            finished: game.finished,
            game_type,
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
                .reserve(Color::Black, game_type),
            reserve_white: state
                .board
                .reserve(Color::White, game_type),
            history: state.history.moves.clone(),
            game_control_history: Self::gc_history(&game.game_control_history),
            white_rating,
            black_rating,
            white_rating_change,
            black_rating_change,
            white_time_left,
            black_time_left,
            white_berserked: game.white_berserked,
            black_berserked: game.black_berserked,
            time_mode,
            time_base: game.time_base,
            time_increment: game.time_increment,
            last_interaction: game.last_interaction,
            finished_at: game.finished_at,
            arena_move_due_at: game.arena_move_due_at,
            speed: game_speed,
            created_at: game.created_at,
            updated_at: game.updated_at,
            conclusion: Conclusion::from_str(&game.conclusion)?,
            repetitions: state.repeating_moves.clone(),
            game_start: GameStart::from_str(&game.game_start)?,
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
