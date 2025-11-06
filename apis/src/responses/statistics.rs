use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SiteStatisticsGamesByTypeResponse {
    pub speed: String,
    pub tournament_games: Option<i64>,
    pub rated_games: Option<i64>,
    pub casual_games: Option<i64>,
    pub total: i64,
    pub period: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SiteStatisticsWinrateByRatingDifferenceResponse {
    pub speed: String,
    pub game_status: String,
    pub bucket: String,
    pub period: String,
    pub num_games: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SiteStatisticsNumberUserRegistrationsResponse {
    pub count: i64,
    pub period: String,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Game,User},
    DbConn,
};
use anyhow::Result;
impl SiteStatisticsGamesByTypeResponse {
    pub async fn get_statistics_games_by_type(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_site_statistics_games_by_type_by_period(conn, period, include_bots).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| SiteStatisticsGamesByTypeResponse {
                speed: stat.speed,
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

impl SiteStatisticsWinrateByRatingDifferenceResponse {
    pub async fn get_site_statistics_winrate_by_rating_difference(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
    ) -> Result<Vec<Self>> {
        let stats = Game::get_site_statistics_winrate_by_rating_difference(conn, period, include_bots).await?;
        let response_stats = stats
            .into_iter()
            .map(|stat| SiteStatisticsWinrateByRatingDifferenceResponse {
                speed: stat.spd,
                game_status: stat.gms,
                bucket: stat.bucket,
                period: stat.period,
                num_games: stat.num_games,
            })
            .collect::<Vec<_>>();
        Ok(response_stats)
    }
}

impl SiteStatisticsNumberUserRegistrationsResponse {
    pub async fn get_number_of_user_registrations(
        conn: &mut DbConn<'_>,
        period: String,
        include_bots: bool,
    ) -> Result<i64> {
        let count = User::get_number_of_user_registrations(conn, period, include_bots).await?;
        Ok(count)
    }
}

}}


