use chrono::{Duration as ChronoDuration, Utc};
use db_lib::{
    get_conn,
    models::{EmailQueueItem, EmailRequestLog, EmailState, EmailToken},
    DbConn,
    DbPool,
};
use std::time::Duration;

const CLEANUP_INTERVAL_SECS: u64 = 60 * 60 * 24;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            match get_conn(&pool).await {
                Ok(mut conn) => {
                    if let Err(err) = cleanup(&mut conn).await {
                        log::warn!("email_cleanup: {err}");
                    }
                }
                Err(err) => log::warn!("email_cleanup: get_conn failed: {err}"),
            }
        }
    });
}

async fn cleanup(conn: &mut DbConn<'_>) -> Result<(), db_lib::db_error::DbError> {
    let now = Utc::now();
    EmailToken::delete_used_before(now - ChronoDuration::days(7), conn).await?;
    EmailToken::delete_expired_before(now - ChronoDuration::days(30), conn).await?;
    EmailRequestLog::delete_before(now - ChronoDuration::days(1), conn).await?;
    EmailQueueItem::prune_sent(now - ChronoDuration::days(30), conn).await?;
    EmailQueueItem::prune_failed(now - ChronoDuration::days(30), conn).await?;
    EmailState::set_cleanup_run(now, conn).await?;
    Ok(())
}
