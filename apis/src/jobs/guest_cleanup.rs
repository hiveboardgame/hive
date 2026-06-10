use chrono::Utc;
use db_lib::{get_conn, models::User, DbPool};
use std::time::Duration;

// Guests are real `users` rows minted lazily when someone starts a game without
// an account. This reaps the abandoned ones (never played) so the table doesn't
// accumulate throwaways. A guest that actually played is kept.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const ABANDON_AFTER: chrono::Duration = chrono::Duration::hours(6);

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(SWEEP_INTERVAL);
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                let cutoff = Utc::now() - ABANDON_AFTER;
                if let Err(e) = User::delete_abandoned_guests(cutoff, &mut conn).await {
                    log::error!("guest_cleanup: {e}");
                }
            }
        }
    });
}
