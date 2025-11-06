use leptos::prelude::*;
use server_fn::codec;
use crate::responses::{
    SiteStatisticsMostActivePlayersByPeriodResponse,
    SiteStatisticsGamesByTypeResponse,
    SiteStatisticsWinrateByRatingDifferenceResponse,
    SiteStatisticsFirstMovesWinrateResponse,
};

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_site_statistics_games_by_type(
    period: String,
    include_bots: bool,
    included_game_types: String,
) -> Result<Vec<SiteStatisticsGamesByTypeResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsGamesByTypeResponse::get_statistics_games_by_type(&mut conn, period, include_bots, included_game_types)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_site_statistics_winrate_by_rating_difference(
    period: String,
    include_bots: bool,
    included_game_types: String,
) -> Result<Vec<SiteStatisticsWinrateByRatingDifferenceResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsWinrateByRatingDifferenceResponse::get_site_statistics_winrate_by_rating_difference(&mut conn, period, include_bots, included_game_types)
        .await
        .map_err(ServerFnError::new)
}


#[server(input = codec::Cbor, output = codec::Cbor)]

pub async fn get_site_statistics_number_user_registrations(
    period: String,
    include_bots: bool,
) -> Result<i64, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use crate::responses::SiteStatisticsNumberUserRegistrationsResponse;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsNumberUserRegistrationsResponse::get_number_of_user_registrations(&mut conn, period, include_bots)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_site_statistics_most_active_players_by_period(
    period: String,
    limit: i64,
    include_bots: bool,
    included_game_types: String,
) -> Result<Vec<SiteStatisticsMostActivePlayersByPeriodResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsMostActivePlayersByPeriodResponse::get_most_active_players_by_period(&mut conn, period, limit, include_bots, included_game_types)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]

pub async fn get_site_statistics_first_moves_winrate(
    period: String,
    include_bots: bool,
    included_game_types: String,
) -> Result<Vec<SiteStatisticsFirstMovesWinrateResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsFirstMovesWinrateResponse::get_first_moves_winrate_statistics(&mut conn, period, include_bots, included_game_types)
        .await
        .map_err(ServerFnError::new)
}