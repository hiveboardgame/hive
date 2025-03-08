use leptos::prelude::*;
use shared_types::TournamentSortOrder;

use crate::responses::{TournamentAbstractResponse, TournamentResponse};
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_all_abstract(sort_order: TournamentSortOrder) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournaments = Tournament::get_all(sort_order, &mut conn).await?;
    let mut result = Vec::new();
    for tournament in tournaments {
        result.push(TournamentAbstractResponse::from_model(&tournament, &mut conn).await.map_err(ServerFnError::new)?);
    }
    Ok(result)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_complete(tournament_id: String) -> Result<TournamentResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament_id = shared_types::TournamentId(tournament_id);
    let tournament = Tournament::find_by_tournament_id(&tournament_id, &mut conn).await?;
    if let Ok(tournament) = TournamentResponse::from_model(&tournament, &mut conn).await {
        Ok(*tournament)
    } else {
        Err(ServerFnError::new("Could not find tournament"))
    }
}
