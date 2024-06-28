use std::time::Duration;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            println!("Checking for tournaments to be started...");
            if let Ok(mut conn) = get_conn(&pool).await {
                let _ = conn.transaction::<_, anyhow::Error, _>(move |tc| {
                    async move {
                        match Tournament::automatic_start(tc).await {
                            Ok(messages) => { for msg in messages {
                                println!("{msg}");
                            }}
                            Err(error) => {
                                println!("{error}");
                            }
                        }
                        Ok(())
                    }
                    .scope_boxed()
                })
                .await;
            }
        }
    });
}
