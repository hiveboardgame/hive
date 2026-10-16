use crate::{
    active_instance::wait_until_active,
    common::{ServerMessage, TournamentUpdate},
    notifications::{notify, Event},
    websocket::{
        server_handlers::{
            game::{append_fixed_field_commit, settle_deadline},
            tournaments::{
                append_public_tournament_patches,
                arena::{append_arena_finalization_effects, append_arena_pairing_effects},
                setup_messages,
                start_outcome_messages,
                PublicTournamentSection,
            },
        },
        HandlerOutput,
        InternalServerMessage,
        MessageDestination,
        WsHub,
    },
};
use actix_web::web::Data;
use anyhow::Result;
use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::Tournament,
    tournaments::{arena, fixed_field},
    DbConn,
    DbPool,
};
use shared_types::{NotificationCategory, TournamentId, TournamentStatus};
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const ARENA_INTERVAL: Duration = Duration::from_secs(1);
const START_BATCH_SIZE: i64 = 100;
const RECONCILIATION_BATCH_SIZE: i64 = 100;
const ARENA_FINALIZATION_BATCH_SIZE: i64 = 100;
const ARENA_PAIRING_BATCH_SIZE: i64 = 100;
const ARENA_TIMEOUT_BATCH_SIZE: i64 = 250;

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    let arena_pool = pool.clone();
    let arena_hub = hub.clone();
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(ARENA_INTERVAL);
        let mut after_id = None;
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            wait_until_active().await;
            if let Err(error) =
                sweep_arena_once(&arena_pool, arena_hub.as_ref(), &mut after_id).await
            {
                log::error!("tournament_runtime: Arena sweep: {error}");
            }
        }
    });
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(SWEEP_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            wait_until_active().await;
            // A failed phase must not stop unrelated tournaments from progressing.
            for (phase, result) in [
                ("expire setups", expire_setups(&pool, hub.as_ref()).await),
                ("reconcile", reconcile(&pool, hub.as_ref()).await),
                (
                    "scheduled starts",
                    start_scheduled(&pool, hub.as_ref()).await,
                ),
                (
                    "scheduled Arena starts",
                    start_scheduled_arenas(&pool, hub.as_ref()).await,
                ),
                (
                    "finalize Arenas",
                    finalize_arenas(&pool, hub.as_ref()).await,
                ),
            ] {
                if let Err(error) = result {
                    log::error!("tournament_runtime: {phase}: {error:?}");
                }
            }
        }
    });
}

async fn expire_setups(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    for expired in
        fixed_field::expire_elimination_setups(Utc::now(), START_BATCH_SIZE, &mut conn).await?
    {
        dispatch_messages(hub, setup_messages(expired, &mut conn).await).await;
    }
    Ok(())
}

async fn reconcile(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let candidates =
        fixed_field::reconciliation_candidates(RECONCILIATION_BATCH_SIZE, &mut conn).await?;
    for tournament_id in candidates {
        match fixed_field::reconcile(tournament_id, &mut conn).await {
            Ok(effects) => {
                let mut output = HandlerOutput::empty();
                append_fixed_field_commit(effects, &mut output, &mut conn).await;
                hub.dispatch_handler_output(output).await;
            }
            Err(error @ DbError::NotFound { .. }) => log::warn!(
                "tournament_runtime: reconcile fixed-field tournament {tournament_id}: {error:?}"
            ),
            Err(error) => log::error!(
                "tournament_runtime: reconcile fixed-field tournament {tournament_id}: {error:?}"
            ),
        }
    }
    Ok(())
}

async fn start_scheduled(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let candidates =
        fixed_field::scheduled_start_candidates(Utc::now(), START_BATCH_SIZE, &mut conn).await?;
    for tournament_id in candidates {
        match fixed_field::start_scheduled(tournament_id, &mut conn).await {
            Ok(outcome) if outcome.started_now => {
                dispatch_messages(
                    hub,
                    start_outcome_messages(
                        outcome.tournament,
                        outcome.games,
                        outcome.removed_invitees,
                        &mut conn,
                    )
                    .await,
                )
                .await
            }
            Ok(outcome) => {
                if outcome.tournament.starts_at.is_none()
                    && outcome.tournament.status() == TournamentStatus::NotStarted
                {
                    scheduled_start_deferred(hub, &outcome.tournament, &mut conn).await;
                }
            }
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: scheduled start {tournament_id}: {error:?}")
            }
            Err(DbError::NotEnoughPlayers) => {}
            Err(error) => {
                log::error!("tournament_runtime: scheduled start {tournament_id}: {error}")
            }
        }
    }
    Ok(())
}

async fn scheduled_start_deferred(
    hub: &Arc<WsHub>,
    tournament: &Tournament,
    conn: &mut DbConn<'_>,
) {
    let mut output = HandlerOutput::empty();
    append_public_tournament_patches(
        tournament.id,
        &[
            PublicTournamentSection::Lifecycle,
            PublicTournamentSection::Memberships,
        ],
        &mut output,
        conn,
    )
    .await;
    hub.dispatch_handler_output(output).await;
    match tournament.organizers(conn).await {
        Ok(organizers) => for organizer in organizers {
            hub.dispatch_message(InternalServerMessage {
                destination: MessageDestination::User(organizer.id),
                message: ServerMessage::Tournament(TournamentUpdate::StartDeferred(TournamentId(tournament.nanoid.clone()))),
            }).await;
            // TODO: i18n once copy is approved.
            notify(Event::TournamentNotice {
                recipient: organizer.id,
                tournament_name: tournament.name.clone(),
                tournament_nanoid: tournament.nanoid.clone(),
                description: String::from("The scheduled start had too few entrants. Start the tournament manually once enough entrants have joined."),
                category: NotificationCategory::Tournament,
            });
        },
        Err(error) => log::error!("Tournament {} changed to manual start but organizer notifications could not be built: {error}", tournament.nanoid),
    }
}

async fn start_scheduled_arenas(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let candidates =
        arena::scheduled_start_candidates(Utc::now(), START_BATCH_SIZE, &mut conn).await?;
    for tournament_id in candidates {
        let presence_hub = Arc::clone(hub);
        match arena::start_scheduled_with_presence(
            tournament_id,
            move |candidates| presence_hub.authenticated_presence_for(candidates),
            &mut conn,
        )
        .await
        {
            Ok(outcome) if outcome.started_now => {
                dispatch_messages(
                    hub,
                    start_outcome_messages(
                        outcome.tournament,
                        outcome.new_games,
                        outcome.removed_invitees,
                        &mut conn,
                    )
                    .await,
                )
                .await
            }
            Ok(_) => {}
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: scheduled Arena start {tournament_id}: {error:?}")
            }
            Err(DbError::NotEnoughPlayers) => {}
            Err(error) => {
                log::error!("tournament_runtime: scheduled Arena start {tournament_id}: {error}")
            }
        }
    }
    Ok(())
}

async fn finalize_arenas(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let candidates =
        arena::cutoff_candidates(Utc::now(), ARENA_FINALIZATION_BATCH_SIZE, &mut conn).await?;
    for tournament_id in candidates {
        // Best effort only: settle deadlines already due before the cutoff,
        // then retain the finalizer's existing commit-visible snapshot rule.
        match arena::due_game_ids(tournament_id, Utc::now(), &mut conn).await {
            Ok(games) => {
                for game_id in games {
                    match settle_deadline(game_id, hub.data.as_ref(), &mut conn).await {
                    Ok(projected) => hub.dispatch_handler_output(projected.output).await,
                    Err(error @ DbError::NotFound { .. }) => log::warn!("tournament_runtime: settle Arena {tournament_id} game {game_id} before finalization: {error:?}"),
                    Err(error) => log::error!("tournament_runtime: settle Arena {tournament_id} game {game_id} before finalization: {error:?}"),
                }
                }
            }
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: due games for Arena {tournament_id}: {error:?}")
            }
            Err(error) => {
                log::error!("tournament_runtime: due games for Arena {tournament_id}: {error}")
            }
        }
        match arena::finalize_due(tournament_id, &mut conn).await {
            Ok(outcome) if outcome.finished_now => {
                let mut output = HandlerOutput::empty();
                append_arena_finalization_effects(
                    outcome.tournament.id,
                    TournamentId(outcome.tournament.nanoid),
                    &mut output,
                    &mut conn,
                )
                .await;
                hub.dispatch_handler_output(output).await;
            }
            Ok(_) => {}
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: finalize Arena {tournament_id}: {error:?}")
            }
            Err(error) => {
                log::error!("tournament_runtime: finalize Arena {tournament_id}: {error}")
            }
        }
    }
    Ok(())
}

async fn sweep_arena_once(
    pool: &DbPool,
    hub: &Arc<WsHub>,
    after_id: &mut Option<Uuid>,
) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let as_of = Utc::now();
    for (phase, candidates) in [
        (
            "opening deadlines",
            arena::opening_deadline_candidates(as_of, ARENA_TIMEOUT_BATCH_SIZE, &mut conn).await,
        ),
        (
            "clock deadlines",
            arena::ordinary_timeout_candidates(as_of, ARENA_TIMEOUT_BATCH_SIZE, &mut conn).await,
        ),
    ] {
        match candidates {
            Ok(games) => {
                for game_id in games {
                    match settle_deadline(game_id, hub.data.as_ref(), &mut conn).await {
                        Ok(projected) => hub.dispatch_handler_output(projected.output).await,
                        Err(error @ DbError::NotFound { .. }) => log::warn!(
                            "tournament_runtime: Arena {phase} game {game_id}: {error:?}"
                        ),
                        Err(error) => {
                            log::error!("tournament_runtime: Arena game {game_id}: {error:?}")
                        }
                    }
                }
            }
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: Arena {phase}: {error:?}")
            }
            Err(error) => log::error!("tournament_runtime: Arena {phase}: {error}"),
        }
    }
    let candidates =
        arena::pairing_candidates(*after_id, Utc::now(), ARENA_PAIRING_BATCH_SIZE, &mut conn)
            .await?;
    if let Some(last) = candidates.last() {
        *after_id = Some(*last);
    }
    for tournament_id in candidates {
        let presence_hub = Arc::clone(hub);
        match arena::pair_waiting_with_presence(
            tournament_id,
            move |candidates| presence_hub.authenticated_presence_for(candidates),
            &mut conn,
        )
        .await
        {
            Ok(outcome) if outcome.changed => {
                let mut output = HandlerOutput::empty();
                append_arena_pairing_effects(
                    tournament_id,
                    &outcome.new_games,
                    &mut output,
                    &mut conn,
                )
                .await;
                hub.dispatch_handler_output(output).await;
            }
            Ok(_) => {}
            Err(error @ DbError::NotFound { .. }) => {
                log::warn!("tournament_runtime: pair Arena {tournament_id}: {error:?}")
            }
            Err(error) => log::error!("tournament_runtime: pair Arena {tournament_id}: {error}"),
        }
    }
    Ok(())
}

pub(crate) async fn dispatch_messages(hub: &WsHub, messages: Vec<InternalServerMessage>) {
    hub.dispatch_handler_output(HandlerOutput::from(messages))
        .await;
}
