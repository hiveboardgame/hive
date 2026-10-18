use crate::{
    responses::{
        tournament::ArenaPairingStatus,
        TournamentAbstractResponse,
        TournamentBrowsePage,
        TournamentCardResponse,
        TournamentCategory,
        TournamentFamily,
        TournamentResponse,
        TournamentTimeFilter,
    },
    security::csrf::CsrfClient,
};
use leptos::prelude::*;
use server_fn::codec;
use shared_types::TournamentId;
use std::collections::HashSet;

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn browse_tournaments(
    category: TournamentCategory,
    search: String,
    page: usize,
    format: Option<TournamentFamily>,
    mode: TournamentTimeFilter,
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
    TournamentBrowsePage::load(category, viewer_id, &search, page, format, mode, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tournament_cards_by_ids(
    tournament_ids: HashSet<TournamentId>,
) -> Result<Vec<TournamentCardResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::get_conn;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let viewer_id = uuid().await.ok();
    TournamentCardResponse::load_by_ids(&tournament_ids, viewer_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_complete(
    tournament_id: TournamentId,
) -> Result<Option<TournamentResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::{Tournament, User},
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
    let viewer_id = uuid().await.ok();
    let administrator = if let Some(viewer_id) = viewer_id {
        User::find_active_by_uuid(&viewer_id, &mut conn)
            .await?
            .admin
    } else {
        false
    };
    let mut response = *TournamentResponse::from_snapshot(snapshot).map_err(ServerFnError::new)?;
    response.memberships = response.memberships.for_viewer(viewer_id, administrator);
    Ok(Some(response))
}

#[cfg(feature = "ssr")]
fn arena_request_error(context: &str, error: impl std::fmt::Display) -> ServerFnError {
    use crate::common::UiError;

    log::error!("Arena request failed while {context}: {error}");
    ServerFnError::new(UiError::RequestUnavailable.encode())
}

#[server(client = CsrfClient)]
pub async fn get_arena_pairing_status(
    tournament_id: TournamentId,
) -> Result<Option<ArenaPairingStatus>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use chrono::Utc;
    use db_lib::{
        get_conn,
        models::{Tournament, TournamentUser},
        schema::games,
    };
    use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
    use diesel_async::RunQueryDsl;
    use shared_types::{tournament::FormatConfig, GameId, TournamentStatus};

    let result: Result<_, ServerFnError> = async {
        let user_id = uuid().await?;
        let pool = pool().await?;
        let mut conn = get_conn(&pool).await?;
        let tournament = Tournament::find_by_tournament_id(&tournament_id, &mut conn).await?;
        let membership = TournamentUser::find(tournament.id, user_id, &mut conn).await?;
        let Some(intent) = membership.pairing_intent()? else {
            return Ok(None);
        };
        let active_game = games::table
            .filter(games::tournament_id.eq(tournament.id))
            .filter(games::finished.eq(false))
            .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id)))
            .select(games::nanoid)
            .first::<String>(&mut conn)
            .await
            .optional()?
            .map(GameId);
        let pairing_seconds_left = match &tournament.configuration().format {
            FormatConfig::Arena(configuration)
                if tournament.status() == TournamentStatus::InProgress =>
            {
                tournament.starts_at.map_or(0, |starts_at| {
                    (configuration.boundaries(starts_at).pairing_closes_at - Utc::now())
                        .num_seconds()
                        .max(0)
                })
            }
            _ => 0,
        };
        Ok(Some(ArenaPairingStatus {
            intent,
            active_game,
            pairing_seconds_left,
        }))
    }
    .await;
    result.map_err(|error| arena_request_error("loading pairing status", error))
}

#[server(client = CsrfClient)]
pub async fn pause_arena(tournament_id: TournamentId) -> Result<(), ServerFnError> {
    use crate::{
        functions::{auth::identity::uuid, db::pool},
        websocket::{server_handlers::tournaments::arena::ArenaHandler, WsHub},
    };
    use actix_web::web::Data;
    use std::sync::Arc;

    let result: Result<_, ServerFnError> = async {
        let user_id = uuid().await?;
        let pool = pool().await?;
        let hub = leptos_actix::extract::<Data<Arc<WsHub>>>().await?;
        let output = ArenaHandler::pause(tournament_id, user_id, Arc::clone(&hub), &pool)
            .handle()
            .await
            .map_err(ServerFnError::new)?;
        hub.dispatch_handler_output(output).await;
        Ok(())
    }
    .await;
    result.map_err(|error| arena_request_error("pausing pairing", error))
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_live_arenas() -> Result<Vec<TournamentAbstractResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use chrono::Utc;
    use db_lib::{get_conn, models::Tournament};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let viewer_id = uuid().await.ok();
    let tournaments = Tournament::get_live_arenas(Utc::now(), &mut conn).await?;
    TournamentAbstractResponse::from_models(&tournaments, viewer_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
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

#[server(client = CsrfClient)]
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
