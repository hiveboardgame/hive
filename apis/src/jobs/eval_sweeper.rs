use chrono::Duration as ChronoDuration;
use db_lib::{get_conn, models::GameEval, DbPool};
use std::time::Duration;
use tokio::time::MissedTickBehavior;

const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
/// Workers report progress every five seconds, so five quiet minutes means the worker is gone.
const STALE_AFTER_MINUTES: i64 = 5;
const MAX_ATTEMPTS: i16 = 3;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(SWEEP_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            crate::active_instance::wait_until_active().await;
            let Ok(mut conn) = get_conn(&pool).await else {
                continue;
            };
            match GameEval::sweep_stale(
                ChronoDuration::minutes(STALE_AFTER_MINUTES),
                MAX_ATTEMPTS,
                &mut conn,
            )
            .await
            {
                Ok((0, 0)) => {}
                Ok((requeued, failed)) => {
                    log::warn!("eval_sweeper: requeued {requeued}, gave up on {failed}")
                }
                Err(e) => log::error!("eval_sweeper: {e}"),
            }
        }
    });
}
