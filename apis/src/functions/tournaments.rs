use crate::responses::{
    TournamentAbstractResponse,
    TournamentBrowsePage,
    TournamentCategory,
    TournamentResponse,
};
use leptos::prelude::*;
use server_fn::codec;
use shared_types::TournamentId;
use std::collections::HashSet;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn browse_tournaments(
    category: TournamentCategory,
    search: String,
    page: usize,
) -> Result<TournamentBrowsePage, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::get_conn;

    let viewer_id = if category.requires_viewer() {
        Some(uuid().await?)
    } else {
        uuid().await.ok()
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    TournamentBrowsePage::load(category, viewer_id, &search, page, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_complete(
    tournament_id: TournamentId,
) -> Result<Option<TournamentResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::Tournament,
        tournaments::public::load_by_id,
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament = match Tournament::find_by_tournament_id(&tournament_id, &mut conn).await {
        Ok(tournament) => tournament,
        Err(DbError::NotFound { .. }) => return Ok(None),
        Err(error) => return Err(ServerFnError::new(error)),
    };
    let snapshot = load_by_id(tournament.id, &mut conn).await?;
    TournamentResponse::from_snapshot(snapshot)
        .map(|tournament| Some(*tournament))
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_live_arenas() -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Tournament};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let viewer_id = uuid().await.ok();
    let tournaments = Tournament::get_live_arenas(&mut conn).await?;
    TournamentAbstractResponse::from_models(&tournaments, viewer_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_abstracts_by_ids(
    tournament_ids: HashSet<TournamentId>,
) -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Tournament};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let viewer_id = uuid().await.ok();
    let tournament_ids: Vec<TournamentId> = tournament_ids.into_iter().collect();
    let tournaments = Tournament::find_by_tournament_ids(&tournament_ids, &mut conn).await?;
    TournamentAbstractResponse::from_models(&tournaments, viewer_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn update_description(
    tournament_id: String,
    description: Option<String>,
) -> Result<Option<String>, ServerFnError> {
    use crate::{
        common::{ServerMessage, TournamentUpdate},
        functions::{auth::identity::uuid, db::pool},
        responses::TournamentPatch,
        websocket::{
            HandlerOutput,
            InternalServerMessage,
            MessageDestination,
            TournamentAudience,
            WsHub,
        },
    };
    use actix_web::web::Data;
    use db_lib::{get_conn, models::Tournament};
    use diesel_async::AsyncConnection;
    use std::sync::Arc;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament_id = TournamentId(tournament_id);
    let user_id = uuid().await?;

    let tournament = conn
        .transaction::<_, ServerFnError, _>(async move |tc| {
            let tournament =
                Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
            Ok(tournament
                .update_description(&user_id, description, tc)
                .await?)
        })
        .await?;
    if let Ok(hub) = leptos_actix::extract::<Data<Arc<WsHub>>>().await {
        let response_id = TournamentId(tournament.nanoid.clone());
        hub.dispatch_handler_output(HandlerOutput::from(vec![InternalServerMessage {
            destination: MessageDestination::Tournament {
                tournament_id: response_id.clone(),
                audience: TournamentAudience::Updates,
            },
            message: ServerMessage::Tournament(TournamentUpdate::Patch {
                tournament_id: response_id,
                patch: Box::new(TournamentPatch::DescriptionChanged(
                    tournament.description.clone(),
                )),
            }),
        }]))
        .await;
    }
    Ok(tournament.description)
}
