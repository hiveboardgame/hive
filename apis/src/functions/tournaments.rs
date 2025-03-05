use leptos::prelude::*;
use shared_types::TournamentSortOrder;

use crate::responses::TournamentAbstractResponse;

#[server]
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
