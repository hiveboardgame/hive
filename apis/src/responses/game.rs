#[cfg(feature = "ssr")]
mod ssr {
    use crate::responses::{tournament::TournamentAbstractResponseDb, user::UserResponseDb};
    use anyhow::Result;
    use db_lib::{models::Game, DbConn};
    use hive_lib::{
        Color,
        GameControl,
        GameStatus::{self, Finished},
        GameType,
        History,
        Piece,
        Position,
        State,
    };
    use shared_types::{
        Conclusion,
        GameBatchResponse,
        GameId,
        GameResponse,
        GameSpeed,
        GameStart,
        GamesQueryOptions,
        TimeMode,
        TournamentAbstractResponse,
        TournamentGameResult,
        UserResponse,
    };
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        time::Duration,
    };
    use uuid::Uuid;

    pub trait GameResponseDb: Sized {
        fn new_from_uuid(
            game_id: Uuid,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn new_from_game_id(
            game_id: &GameId,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_model(
            game: &Game,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn batch_from_options(
            options: GamesQueryOptions,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<GameBatchResponse>> + Send;
        fn from_game_ids(
            game_ids: &[Uuid],
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Vec<Self>>> + Send;
        fn from_games_batch(
            games: Vec<Game>,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Vec<Self>>> + Send;
    }

    impl GameResponseDb for GameResponse {
        async fn new_from_uuid(game_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
            let game = Game::find_by_uuid(&game_id, conn).await?;
            GameResponse::from_model(&game, conn).await
        }

        async fn new_from_game_id(game_id: &GameId, conn: &mut DbConn<'_>) -> Result<Self> {
            let game = Game::find_by_game_id(game_id, conn).await?;
            GameResponse::from_model(&game, conn).await
        }

        async fn from_model(game: &Game, conn: &mut DbConn<'_>) -> Result<Self> {
            let history = Box::new(History::new_from_str(&game.history)?);
            let state = Box::new(State::new_from_history(&history)?);
            new_from(game, state, conn).await
        }

        async fn batch_from_options(
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

        async fn from_game_ids(game_ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
            let games = Game::find_by_game_ids(game_ids, conn).await?;
            Self::from_games_batch(games, conn).await
        }

        async fn from_games_batch(games: Vec<Game>, conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
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
                let white_player = users_map.get(&game.white_id).cloned().ok_or_else(|| {
                    anyhow::anyhow!("White player not found for game {}", game.id)
                })?;
                let black_player = users_map.get(&game.black_id).cloned().ok_or_else(|| {
                    anyhow::anyhow!("Black player not found for game {}", game.id)
                })?;

                let tournament = game.tournament_id.and_then(|tid| tournaments_map.get(&tid));

                let history = Box::new(History::new_from_str(&game.history)?);
                let state = Box::new(State::new_from_history(&history)?);

                result.push(
                    new_from_batch(
                        &game,
                        state,
                        white_player,
                        black_player,
                        tournament.cloned(),
                    )
                    .await?,
                );
            }

            Ok(result)
        }
    }

    async fn new_from(
        game: &Game,
        state: Box<State>,
        conn: &mut DbConn<'_>,
    ) -> Result<GameResponse> {
        let white_player = UserResponse::from_uuid(&game.white_id, conn).await?;
        let black_player = UserResponse::from_uuid(&game.black_id, conn).await?;
        let tournament = if let Some(tournament_id) = game.tournament_id {
            Some(TournamentAbstractResponse::from_uuid(&tournament_id, conn).await?)
        } else {
            None
        };

        new_from_batch(game, state, white_player, black_player, tournament).await
    }

    async fn new_from_batch(
        game: &Game,
        state: Box<State>,
        white_player: UserResponse,
        black_player: UserResponse,
        tournament: Option<TournamentAbstractResponse>,
    ) -> Result<GameResponse> {
        let (white_rating, black_rating, white_rating_change, black_rating_change) = {
            if let Finished(_) = GameStatus::from_str(&game.game_status)? {
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
        let white_time_left = game
            .white_time_left
            .map(|nanos| Duration::from_nanos(nanos as u64));
        let black_time_left = game
            .black_time_left
            .map(|nanos| Duration::from_nanos(nanos as u64));
        Ok(GameResponse {
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
            moves: moves_as_string(state.board.moves(state.turn_color)),
            spawns: state
                .board
                .spawnable_positions(state.turn_color)
                .collect::<Vec<_>>(),
            rated: game.rated,
            reserve_black: state.board.reserve(
                Color::Black,
                game.game_type.parse().expect("Gametype parsed"),
            ),
            reserve_white: state.board.reserve(
                Color::White,
                game.game_type.parse().expect("Gametype parsed"),
            ),
            history: state.history.moves.clone(),
            game_control_history: gc_history(&game.game_control_history),
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
                turn = turn_str
                    .strip_suffix('.')
                    .expect("Suffix exists")
                    .parse()
                    .expect("Turn parsed");
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

#[cfg(feature = "ssr")]
pub use ssr::GameResponseDb;
