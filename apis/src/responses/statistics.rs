use crate::responses::user::UserResponse;
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GamesByTypeResponse {
    pub speed: GameSpeed,
    pub tournament_games: Option<i64>,
    pub rated_games: Option<i64>,
    pub casual_games: Option<i64>,
    pub total: i64,
    pub period: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct WinrateByRatingDifferenceResponse {
    pub speed: GameSpeed,
    pub game_status: String,
    pub bucket: String,
    pub period: String,
    pub num_games: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NumberUserRegistrationsResponse {
    pub count: i64,
    pub period: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MostActivePlayersByPeriodResponse {
    pub speed: GameSpeed,
    pub period: String,
    pub user_resp: UserResponse,
    pub num_games: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FirstMovesWinrateResponse {
    pub speed: GameSpeed,
    pub first_moves: String,
    pub period: String,
    pub white_wins: i64,
    pub black_wins: i64,
    pub draws: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatingBucketsResponse {
    pub speed: GameSpeed,
    pub bucket: i32,
    pub number_of_players: i64,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Game,User,Rating},
    DbConn,
};
use anyhow::Result;
use shared_types::GameTypeFilter;

impl GamesByTypeResponse {
    pub async fn get_statistics_games_by_type(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
        included_game_types: GameTypeFilter,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_games_by_type_by_period(conn, period, include_bots, included_game_types).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| GamesByTypeResponse {
                speed: stat.speed.as_str().parse().unwrap_or(GameSpeed::AllSpeeds),
                tournament_games: stat.tournament_games,
                rated_games: stat.rated_games,
                casual_games: stat.casual_games,
                total: stat.total,
                period: stat.period,
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

impl WinrateByRatingDifferenceResponse {
    pub async fn get_winrate_by_rating_difference(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
        included_game_types: GameTypeFilter,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_winrate_by_rating_difference(conn, period, include_bots, included_game_types).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| WinrateByRatingDifferenceResponse {
                speed: stat.spd.as_str().parse().unwrap_or(GameSpeed::AllSpeeds),
                game_status: stat.gms,
                bucket: stat.bucket,
                period: stat.period,
                num_games: stat.num_games,
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

impl MostActivePlayersByPeriodResponse {
    pub async fn get_most_active_players_by_period(
        conn: &mut DbConn<'_>,
        period: String,
        limit: i64,
        include_bots: bool,
        included_game_types: GameTypeFilter,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_most_active_players_by_period(conn, period, limit, include_bots, included_game_types).await?;
        let user_responses = UserResponse::from_uuids(
            &stats.iter().map(|stat| stat.user_id).collect::<Vec<_>>(),
            conn,
        ).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| MostActivePlayersByPeriodResponse {
                speed: stat.spd.as_str().parse().unwrap_or(GameSpeed::AllSpeeds),
                period: stat.period,
                user_resp: user_responses.get(&stat.user_id)
                    .cloned()
                    .expect("User response should exist for user_id"),
                num_games: stat.num_games,
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

impl NumberUserRegistrationsResponse {
    pub async fn get_number_of_user_registrations(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
    ) -> Result<i64> {
        let count = User::get_number_of_user_registrations(conn, period, include_bots).await?;
        Ok(count)
    }
}

impl FirstMovesWinrateResponse {
    pub async fn get_first_moves_winrate_statistics(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
        included_game_types: GameTypeFilter,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_first_moves_winrate(conn, period, include_bots, included_game_types, 10).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| FirstMovesWinrateResponse {
                speed: stat.spd.as_str().parse().unwrap_or(GameSpeed::AllSpeeds),
                first_moves: stat.first_moves,
                period: stat.period,
                white_wins: stat.white_wins,
                black_wins: stat.black_wins,
                draws: stat.draws,
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

impl RatingBucketsResponse {
    pub async fn get_rating_buckets(
        conn: &mut DbConn<'_>,
        include_bots: bool,
    ) -> Result<Vec<Self>> {
        let stats = Rating::get_rating_buckets(conn, 10, include_bots).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| {
                RatingBucketsResponse {
                    speed: stat.spd.as_str().parse().unwrap_or(GameSpeed::AllSpeeds),
                    bucket: stat.bucket,
                    number_of_players: stat.number_of_players,
                }
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

}}
