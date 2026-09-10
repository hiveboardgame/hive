use crate::websocket::{server_handlers::game::settle_deadline, WsHub};
use actix_web::web::Data;
use anyhow::{Error, Result};
use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::Game,
    tournaments::{arena, fixed_field},
    DbConn,
    DbPool,
};
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

/// The partial-index query is near-free, so frequency is bounded only by
/// how late a no-viewer timeout flag is allowed to arrive.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const SWEEP_BATCH_SIZE: i64 = 250;

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(SWEEP_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(e) = sweep_once(&pool, hub.as_ref()).await {
                log::error!("timeout_sweeper: {e}");
            }
        }
    });
}

async fn sweep_once(pool: &DbPool, hub: &Arc<WsHub>) -> Result<()> {
    let mut conn = get_conn(pool).await?;
    let as_of = Utc::now();
    let expired = Game::find_expired_by_timeout_at(as_of, SWEEP_BATCH_SIZE, &mut conn).await?;
    for game in expired {
        let nanoid = game.nanoid.clone();
        if let Err(e) = sweep_game_one(game.id, hub, &mut conn).await {
            log::error!("timeout_sweeper: game {nanoid}: {e}");
        }
    }
    let tournament_expired =
        fixed_field::timeout_candidates(as_of, SWEEP_BATCH_SIZE, &mut conn).await?;
    for game_id in tournament_expired {
        if let Err(error) = sweep_game_one(game_id, hub, &mut conn).await {
            if !stale_tournament_timeout(&error) {
                log::error!("timeout_sweeper: tournament game {game_id}: {error}");
            }
        }
    }
    let arena_opening =
        arena::opening_deadline_candidates(as_of, SWEEP_BATCH_SIZE, &mut conn).await?;
    let arena_ordinary =
        arena::ordinary_timeout_candidates(as_of, SWEEP_BATCH_SIZE, &mut conn).await?;
    for game_id in arena_opening.into_iter().chain(arena_ordinary) {
        if let Err(error) = sweep_game_one(game_id, hub, &mut conn).await {
            if !stale_tournament_timeout(&error) {
                log::error!("timeout_sweeper: Arena game {game_id}: {error}");
            }
        }
    }
    Ok(())
}

fn stale_tournament_timeout(error: &Error) -> bool {
    error
        .downcast_ref::<DbError>()
        .is_some_and(|error| matches!(error, DbError::GameIsOver | DbError::NotFound { .. }))
}

async fn sweep_game_one(game_id: Uuid, hub: &Arc<WsHub>, conn: &mut DbConn<'_>) -> Result<()> {
    let projected = settle_deadline(game_id, hub.data.as_ref(), conn).await?;
    hub.dispatch_handler_output(projected.output).await;
    Ok(())
}
