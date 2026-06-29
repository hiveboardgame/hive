use chrono::{Duration as ChronoDuration, Utc};
use db_lib::{get_conn, models::PushDevice, DbPool};
use std::time::Duration;

const STALE_THRESHOLD_DAYS: i64 = 90;

const SWEEP_INTERVAL_SECS: u64 = 60 * 60 * 24;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(SWEEP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let threshold = Utc::now() - ChronoDuration::days(STALE_THRESHOLD_DAYS);
            match get_conn(&pool).await {
                Ok(mut conn) => match PushDevice::delete_stale(threshold, &mut conn).await {
                    Ok(0) => {}
                    Ok(n) => log::info!("push_device_sweep: pruned {n} stale device(s)"),
                    Err(err) => log::warn!("push_device_sweep: delete_stale failed: {err}"),
                },
                Err(err) => log::warn!("push_device_sweep: get_conn failed: {err}"),
            }
        }
    });
}
