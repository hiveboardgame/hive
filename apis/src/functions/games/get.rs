use crate::functions::games::get::server_fn::codec;
use crate::responses::GameResponse;
use crate::responses::{
    SiteStatisticsGamesByTypeResponse,
    SiteStatisticsWinrateByRatingDifferenceResponse,
};
use leptos::prelude::*;
use shared_types::{GameId, GamesQueryOptions};
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_uuid(game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_nanoid(game_id: GameId) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_game_id(&game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_batch_from_options(
    options: GamesQueryOptions,
) -> Result<Vec<GameResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::vec_from_options(options, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_site_statistics_games_by_type(
    period: String,
    include_bots: bool,
) -> Result<Vec<SiteStatisticsGamesByTypeResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsGamesByTypeResponse::get_statistics_games_by_type(&mut conn, period, include_bots)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_site_statistics_winrate_by_rating_difference(
    period: String,
    include_bots: bool,
) -> Result<Vec<SiteStatisticsWinrateByRatingDifferenceResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    SiteStatisticsWinrateByRatingDifferenceResponse::get_site_statistics_winrate_by_rating_difference(&mut conn, period, include_bots)
        .await
        .map_err(ServerFnError::new)
}