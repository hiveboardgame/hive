use crate::responses::{TournamentAbstractResponse, TournamentResponse};
use leptos::prelude::*;
use server_fn::codec;
use shared_types::{TournamentId, TournamentSortOrder, TournamentStatus};
use std::collections::HashSet;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_all_abstract(
    sort_order: TournamentSortOrder,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournaments = Tournament::get_all(sort_order, &mut conn).await?;
    let mut result = Vec::new();
    for tournament in tournaments {
        result.push(
            TournamentAbstractResponse::from_model(&tournament, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        );
    }
    Ok(result)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_complete(
    tournament_id: TournamentId,
) -> Result<TournamentResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament = Tournament::find_by_tournament_id(&tournament_id, &mut conn).await?;
    if let Ok(tournament) = TournamentResponse::from_model(&tournament, &mut conn).await {
        Ok(*tournament)
    } else {
        Err(ServerFnError::new("Could not find tournament"))
    }
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_by_status(
    status: TournamentStatus,
    sort_order: TournamentSortOrder,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournaments = Tournament::get_by_status(status, sort_order, &mut conn).await?;
    let mut result = Vec::new();
    for tournament in tournaments {
        result.push(
            TournamentAbstractResponse::from_model(&tournament, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        );
    }
    Ok(result)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_abstracts_by_ids(
    tournament_ids: HashSet<TournamentId>,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament_ids: Vec<TournamentId> = tournament_ids.into_iter().collect();
    let tournaments = Tournament::find_by_tournament_ids(&tournament_ids, &mut conn).await?;
    let mut result = Vec::new();
    //TODO: This makes N database calls - should use batch queries for better performance
    for tournament in tournaments {
        result.push(
            TournamentAbstractResponse::from_model(&tournament, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        );
    }
    Ok(result)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_hosting_tournaments(
    sort_order: TournamentSortOrder,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user_id = uuid().await?;
    let tournaments = Tournament::get_hosting_tournaments(&user_id, sort_order, &mut conn).await?;
    let mut result = Vec::new();
    for tournament in tournaments {
        result.push(
            TournamentAbstractResponse::from_model(&tournament, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        );
    }
    Ok(result)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_joined_tournaments(
    sort_order: TournamentSortOrder,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user_id = uuid().await?;
    let tournaments = Tournament::get_joined_tournaments(&user_id, sort_order, &mut conn).await?;
    let mut result = Vec::new();
    for tournament in tournaments {
        result.push(
            TournamentAbstractResponse::from_model(&tournament, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        );
    }
    Ok(result)
}

#[server]
pub async fn update_description(
    tournament_id: String,
    description: String,
) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Tournament;

    if description.len() < 50 {
        return Err(ServerFnError::new(
            "Description must be at least 50 characters long",
        ));
    }

    if description.len() > 2000 {
        return Err(ServerFnError::new(
            "Description must be at most 2000 characters long",
        ));
    }

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament_id = shared_types::TournamentId(tournament_id);
    let user_id = uuid().await?;

    let tournament = Tournament::find_by_tournament_id(&tournament_id, &mut conn).await?;
    tournament
        .update_description(&user_id, &description, &mut conn)
        .await?;

    Ok(())
}
