use db_lib::{get_conn, models::Challenge, DbPool};
use std::time::Duration;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60 * 60 * 24 * 7));
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                let _ = Challenge::delete_old_non_public_challenges(&mut conn).await;
            }
        }
    });
}
