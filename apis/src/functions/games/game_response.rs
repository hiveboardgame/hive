use serde::{Deserialize, Serialize};
use uuid::Uuid;
// use serde_with::serde_as;
use crate::functions::users::user_response::UserResponse;
use hive_lib::{
    bug::Bug, game_control::GameControl, game_result::GameResult, game_status::GameStatus,
    game_type::GameType, history::History, position::Position, state::State,
};
use std::collections::HashMap;

// #[serde_as]
// #[serde_with::skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GameStateResponse {
    pub game_id: Uuid,
    pub nanoid: String,
    pub turn: usize,
    pub game_status: GameStatus,
    pub game_type: GameType,
    pub tournament_queen_rule: bool,
    pub white_player: UserResponse,
    pub black_player: UserResponse,
    // #[serde_as(as = "Vec<(_, _)>")]
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
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{game::Game, rating::Rating},
    DbPool,
};
use hive_lib::{
    color::Color, game_status::GameStatus::Finished, piece::Piece,
};
use leptos::*;
use std::str::FromStr;

impl GameStateResponse {
    pub async fn new_from_nanoid(game_id: &str, pool: &DbPool) -> Result<Self, ServerFnError> {
        let game = Game::find_by_nanoid(game_id, pool).await?;
        GameStateResponse::new_from_db(&game, pool).await
    }

    pub async fn new_from_db(game: &Game, pool: &DbPool) -> Result<Self, ServerFnError> {
        let history = History::new_from_str(&game.history)?;
        let state = State::new_from_history(&history)?;
        GameStateResponse::new_from(game, &state, pool).await
    }

    pub async fn new_from(
        game: &Game,
        state: &State,
        pool: &DbPool,
    ) -> Result<Self, ServerFnError> {
        let (white_rating, black_rating, white_rating_change, black_rating_change) = {
            if let Finished(_) = GameStatus::from_str(&game.game_status).unwrap() {
                (
                    game.white_rating,
                    game.black_rating,
                    game.white_rating_change,
                    game.black_rating_change,
                )
            } else {
                (
                    Some(Rating::for_uuid(&game.white_id, pool).await?.rating),
                    Some(Rating::for_uuid(&game.black_id, pool).await?.rating),
                    None,
                    None,
                )
            }
        };
        Ok(Self {
            game_id: game.id,
            nanoid: game.nanoid.clone(),
            game_status: GameStatus::from_str(&game.game_status)?,
            game_type: GameType::from_str(&game.game_type)?,
            tournament_queen_rule: state.tournament,
            turn: state.turn,
            white_player: UserResponse::from_uuid(&game.white_id, pool).await?,
            black_player: UserResponse::from_uuid(&game.black_id, pool).await?,
            moves: GameStateResponse::moves_as_string(state.board.moves(state.turn_color)),
            spawns: state
                .board
                .spawnable_positions(state.turn_color)
                .collect::<Vec<_>>(),
            rated: game.rated,
            reserve_black: state
                .board
                .reserve(Color::Black, game.game_type.parse().unwrap()),
            reserve_white: state
                .board
                .reserve(Color::White, game.game_type.parse().unwrap()),
            history: state.history.moves.clone(),
            game_control_history: Self::gc_history(&game.game_control_history),
            white_rating,
            black_rating,
            white_rating_change,
            black_rating_change,
        })
    }

    fn gc_history(gcs: &str) -> Vec<(i32, GameControl)> {
        let mut ret = Vec::new();
        for gc_str in gcs.split_terminator(';') {
            let turn: i32;
            let gc: GameControl;
            // TODO: This code is janky
            if let Some(turn_str) = gc_str.split(' ').next() {
                turn = turn_str.strip_suffix('.').unwrap().parse().unwrap();
                if let Some(gc_token) = gc_str.split(' ').nth(1) {
                    gc = gc_token.parse().unwrap();
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

impl GameStateResponse {
    pub fn create_state(self) -> State {
        let result = match self.game_status {
            GameStatus::NotStarted | GameStatus::InProgress => GameResult::Unknown,
            GameStatus::Finished(result) => result,
        };
        State::new_from_history(&History::new_from_gamestate(
            self.history,
            result,
            self.game_type,
        ))
        .expect("State to be valid, as game was")
    }
}
