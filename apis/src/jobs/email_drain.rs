use crate::email::{deliver, render_password_reset, EmailConfig};
use chrono::{Duration as ChronoDuration, Utc};
use db_lib::{get_conn, models::EmailQueueItem, DbConn, DbPool};
use std::time::Duration;
use tokio::time::MissedTickBehavior;

const DRAIN_INTERVAL_SECS: u64 = 3;
const BATCH_SIZE: i64 = 50;
const MAX_ATTEMPTS: i16 = 3;

pub fn run(pool: DbPool, config: EmailConfig) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(DRAIN_INTERVAL_SECS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            match get_conn(&pool).await {
                Ok(mut conn) => match EmailQueueItem::claim_batch(BATCH_SIZE, &mut conn).await {
                    Ok(batch) => {
                        for item in batch {
                            process(&config, item, &mut conn).await;
                        }
                    }
                    Err(err) => log::warn!("email_drain: claim_batch failed: {err}"),
                },
                Err(err) => log::warn!("email_drain: get_conn failed: {err}"),
            }
        }
    });
}

async fn process(config: &EmailConfig, item: EmailQueueItem, conn: &mut DbConn<'_>) {
    let Some((subject, body)) = render(config, &item) else {
        let _ = EmailQueueItem::mark_skipped(item.id, "skipped: cannot render", conn).await;
        return;
    };
    match deliver(config, &item.to_address, &subject, &body).await {
        Ok(()) => {
            if let Err(err) = EmailQueueItem::mark_sent(item.id, conn).await {
                log::warn!("email_drain: mark_sent failed for {}: {err}", item.id);
            }
        }
        Err(err) => {
            let attempts = item.attempts + 1;
            if attempts >= MAX_ATTEMPTS {
                log::error!(
                    "email_deadletter: id={} to={} kind={} error={}",
                    item.id,
                    item.to_address,
                    item.kind,
                    err
                );
            }
            let backoff = ChronoDuration::minutes(2_i64.saturating_pow(attempts as u32));
            let next_at = Utc::now() + backoff;
            if let Err(e) =
                EmailQueueItem::mark_failed(item.id, attempts, &err, next_at, conn).await
            {
                log::warn!("email_drain: mark_failed failed for {}: {e}", item.id);
            }
        }
    }
}

fn render(config: &EmailConfig, item: &EmailQueueItem) -> Option<(String, String)> {
    match item.kind.as_str() {
        "password_reset" => {
            let token = item.payload.get("token")?.as_str()?;
            let username = item
                .payload
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("there");
            Some(render_password_reset(&config.base_url, username, token))
        }
        _ => None,
    }
}
