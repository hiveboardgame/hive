#![allow(dead_code)]
use db_lib::{get_conn, DbPool};
use diesel::{
    sql_types::{Bool, Integer},
    QueryableByName,
};
use diesel_async::RunQueryDsl;

#[derive(QueryableByName)]
pub struct Backend {
    #[diesel(sql_type = Integer)]
    pub pid: i32,
}

#[derive(QueryableByName)]
struct LockWait {
    #[diesel(sql_type = Bool)]
    blocked: bool,
}

pub async fn wait_for_backend_lock(pool: &DbPool, pid: i32) {
    let mut observer = get_conn(pool).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let wait = diesel::sql_query("SELECT cardinality(pg_blocking_pids($1)) > 0 AS blocked")
                .bind::<Integer, _>(pid)
                .get_result::<LockWait>(&mut observer)
                .await
                .unwrap();
            if wait.blocked {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("contender reached its database lock");
}
