use crate::responses::{
    FirstMovesWinrateResponse, GamesByTypeResponse, MostActivePlayersByPeriodResponse,
    RatingBucketsResponse, WinrateByRatingDifferenceResponse,
};
use leptos::prelude::*;
use server_fn::codec;
use shared_types::GameTypeFilter;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_games_by_type(
    period: String,
    include_bots: bool,
    included_game_types: GameTypeFilter,
) -> Result<Vec<GamesByTypeResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GamesByTypeResponse::get_statistics_games_by_type(
        &mut conn,
        period,
        include_bots,
        included_game_types,
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_winrate_by_rating_difference(
    period: String,
    include_bots: bool,
    included_game_types: GameTypeFilter,
) -> Result<Vec<WinrateByRatingDifferenceResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    WinrateByRatingDifferenceResponse::get_winrate_by_rating_difference(
        &mut conn,
        period,
        include_bots,
        included_game_types,
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]

pub async fn get_number_user_registrations(
    period: String,
    include_bots: bool,
) -> Result<i64, ServerFnError> {
    use crate::functions::db::pool;
    use crate::responses::NumberUserRegistrationsResponse;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    NumberUserRegistrationsResponse::get_number_of_user_registrations(
        &mut conn,
        period,
        include_bots,
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_most_active_players_by_period(
    period: String,
    limit: i64,
    include_bots: bool,
    included_game_types: GameTypeFilter,
) -> Result<Vec<MostActivePlayersByPeriodResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    MostActivePlayersByPeriodResponse::get_most_active_players_by_period(
        &mut conn,
        period,
        limit,
        include_bots,
        included_game_types,
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]

pub async fn get_first_moves_winrate(
    period: String,
    include_bots: bool,
    included_game_types: GameTypeFilter,
) -> Result<Vec<FirstMovesWinrateResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    FirstMovesWinrateResponse::get_first_moves_winrate_statistics(
        &mut conn,
        period,
        include_bots,
        included_game_types,
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_rating_buckets(
    include_bots: bool,
) -> Result<Vec<RatingBucketsResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    RatingBucketsResponse::get_rating_buckets(&mut conn, include_bots)
        .await
        .map_err(ServerFnError::new)
}
