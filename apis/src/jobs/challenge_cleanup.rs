use db_lib::{get_conn, models::Challenge, DbPool};
use diesel_async::AsyncConnection;
use std::time::Duration;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60 * 60 * 24));
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                let _ = conn
                    .transaction::<_, anyhow::Error, _>(async move |tc| {
                        if !crate::jobs::try_advisory_xact_lock(
                            tc,
                            crate::jobs::CHALLENGE_CLEANUP_LOCK,
                        )
                        .await?
                        {
                            return Ok(());
                        }
                        Challenge::delete_old_non_public_challenges(tc).await?;
                        Ok(())
                    })
                    .await;
            }
        }
    });
}
