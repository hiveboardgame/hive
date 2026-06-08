use crate::websocket::WsHub;
use actix_web::web::Data;
use chrono::Utc;
use db_lib::{get_conn, models::Game, DbConn, DbPool};
use shared_types::Conclusion;
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;

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

async fn sweep_once(pool: &DbPool, hub: &Arc<WsHub>) -> anyhow::Result<()> {
    let mut conn = get_conn(pool).await?;
    let expired = Game::find_expired_by_timeout_at(Utc::now(), SWEEP_BATCH_SIZE, &mut conn).await?;
    for game in expired {
        let nanoid = game.nanoid.clone();
        if let Err(e) = sweep_one(game, hub, &mut conn).await {
            log::error!("timeout_sweeper: game {nanoid}: {e}");
        }
    }
    Ok(())
}

async fn sweep_one(game: Game, hub: &Arc<WsHub>, conn: &mut DbConn<'_>) -> anyhow::Result<()> {
    // Idempotent: a concurrent move that reset the clock returns the row
    // unchanged. Reuses the shared finalize path instead of duplicating it.
    let finalized = game.check_time(conn).await?;
    // Another path (resign/draw/control) may have finalized this row in the
    // window since the batch query. Only broadcast when the timeout is what
    // ended it — otherwise we'd label the wrong loser.
    if finalized.conclusion != Conclusion::Timeout.to_string() {
        return Ok(());
    }
    hub.broadcast_timeout_finalize(conn, &finalized).await
}
