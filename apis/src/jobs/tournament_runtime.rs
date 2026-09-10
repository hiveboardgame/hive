use crate::websocket::{
    server_handlers::{
        game::append_fixed_field_commit,
        tournaments::{
            arena::{append_arena_finalization_effects, append_arena_pairing_effects},
            setup_messages,
            start_outcome_messages,
        },
    },
    HandlerOutput,
    InternalServerMessage,
    WsHub,
};
use actix_web::web::Data;
use anyhow::Result;
use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    tournaments::{arena, fixed_field},
    DbPool,
};
use shared_types::TournamentId;
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const START_BATCH_SIZE: i64 = 100;
const RECONCILIATION_BATCH_SIZE: i64 = 100;
const ARENA_FINALIZATION_BATCH_SIZE: i64 = 100;
const ARENA_PAIRING_BATCH_SIZE: i64 = 100;

#[derive(Default)]
struct ArenaPairingCursor {
    after_id: Option<Uuid>,
}

impl ArenaPairingCursor {
    fn observe(&mut self, candidates: &[Uuid]) {
        if let Some(last) = candidates.last() {
            self.after_id = Some(*last);
        }
    }
}

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(SWEEP_INTERVAL);
        let mut arena_cursor = ArenaPairingCursor::default();
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(error) = sweep_once(&pool, hub.as_ref(), &mut arena_cursor).await {
                log::error!("tournament_runtime: {error}");
            }
        }
    });
}

async fn sweep_once(
    pool: &DbPool,
    hub: &Arc<WsHub>,
    arena_cursor: &mut ArenaPairingCursor,
) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let as_of = Utc::now();
    for expired in
        fixed_field::expire_elimination_setups(as_of, START_BATCH_SIZE, &mut conn).await?
    {
        dispatch_messages(hub, setup_messages(expired, &mut conn).await).await;
    }
    let reconciliation_candidates =
        fixed_field::reconciliation_candidates(RECONCILIATION_BATCH_SIZE, &mut conn).await?;
    for tournament_id in reconciliation_candidates {
        match fixed_field::reconcile(tournament_id, &mut conn).await {
            Ok(effects) => {
                let mut output = HandlerOutput::empty();
                append_fixed_field_commit(effects, &mut output, &mut conn).await;
                hub.dispatch_handler_output(output).await;
            }
            Err(error) => {
                log::error!(
                    "tournament_runtime: reconcile fixed-field tournament {tournament_id}: {error}"
                );
            }
        }
    }

    let scheduled =
        fixed_field::scheduled_start_candidates(as_of, START_BATCH_SIZE, &mut conn).await?;
    for tournament_id in scheduled {
        let start = fixed_field::start_scheduled(tournament_id, &mut conn)
            .await
            .map(|outcome| {
                (
                    outcome.tournament,
                    outcome.games,
                    outcome.removed_invitees,
                    outcome.started_now,
                )
            });
        match start {
            Ok((tournament, games, removed_invitees, started_now)) => {
                let messages = if started_now {
                    start_outcome_messages(tournament, games, removed_invitees, &mut conn).await
                } else {
                    Vec::new()
                };
                dispatch_messages(hub, messages).await;
            }
            Err(error) if expected_start_deferral(&error) => {}
            Err(error) => {
                log::error!("tournament_runtime: scheduled start {tournament_id}: {error}");
            }
        }
    }

    let scheduled_arenas =
        arena::scheduled_start_candidates(as_of, START_BATCH_SIZE, &mut conn).await?;
    for tournament_id in scheduled_arenas {
        let presence_hub = Arc::clone(hub);
        let start = arena::start_scheduled_with_presence(
            tournament_id,
            move |candidates| presence_hub.authenticated_presence_for(candidates),
            &mut conn,
        )
        .await;
        match start {
            Ok(outcome) => {
                let messages = if outcome.started_now {
                    start_outcome_messages(
                        outcome.tournament,
                        outcome.new_games,
                        outcome.removed_invitees,
                        &mut conn,
                    )
                    .await
                } else {
                    Vec::new()
                };
                dispatch_messages(hub, messages).await;
            }
            Err(error) if expected_start_deferral(&error) => {}
            Err(error) => {
                log::error!("tournament_runtime: scheduled Arena start {tournament_id}: {error}");
            }
        }
    }

    let cutoff_candidates =
        arena::cutoff_candidates(as_of, ARENA_FINALIZATION_BATCH_SIZE, &mut conn).await?;
    for tournament_id in cutoff_candidates {
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
            Err(error) if stale_arena_pairing_error(&error) => {}
            Err(error) => {
                log::error!("tournament_runtime: finalize Arena {tournament_id}: {error}");
            }
        }
    }

    let pairing_candidates = arena::pairing_candidates(
        arena_cursor.after_id,
        as_of,
        ARENA_PAIRING_BATCH_SIZE,
        &mut conn,
    )
    .await?;
    arena_cursor.observe(&pairing_candidates);
    for tournament_id in pairing_candidates {
        let presence_hub = Arc::clone(hub);
        let pairing = arena::pair_waiting_with_presence(
            tournament_id,
            move |candidates| presence_hub.authenticated_presence_for(candidates),
            &mut conn,
        )
        .await;
        match pairing {
            Ok(outcome) => {
                if !outcome.changed {
                    continue;
                }
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
            Err(error) if stale_arena_pairing_error(&error) => {}
            Err(error) => {
                log::error!("tournament_runtime: pair Arena {tournament_id}: {error}");
            }
        }
    }

    Ok(())
}

fn expected_start_deferral(error: &DbError) -> bool {
    matches!(error, DbError::NotEnoughPlayers | DbError::NotFound { .. })
}

fn stale_arena_pairing_error(error: &DbError) -> bool {
    matches!(error, DbError::NotFound { .. } | DbError::GameIsOver)
}

pub(crate) async fn dispatch_messages(hub: &WsHub, messages: Vec<InternalServerMessage>) {
    for message in messages {
        if let Err(error) = hub.dispatch_message(message).await {
            log::error!("tournament_runtime: encode committed effect: {error}");
        }
    }
}
