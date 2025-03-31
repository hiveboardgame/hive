use bb8::PooledConnection;
use diesel::result::{Error as DieselError, Error::QueryBuilderError};
use diesel_async::{
    pg::AsyncPgConnection,
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, PoolError},
};

pub mod config;
pub mod db_error;
pub mod models;
pub mod schema;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn<'a> = PooledConnection<'a, AsyncDieselConnectionManager<AsyncPgConnection>>;

pub async fn get_pool(db_uri: &str) -> Result<DbPool, PoolError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_uri);
    Pool::builder().build(manager).await
}

pub async fn get_conn(pool: &DbPool) -> Result<DbConn, DieselError> {
    pool.get().await.map_err(|e| QueryBuilderError(e.into()))
}

#[cfg(test)]
pub mod test_utils {
    use crate::DbConn;
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use diesel_async::AsyncPgConnection;
    use diesel_async::SimpleAsyncConnection;
    use std::env;
    use std::sync::OnceLock;

    // Helper to get database URL
    fn get_test_db_url() -> String {
        dotenvy::dotenv().ok();
        env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
    }

    // Static function to initialize the pool
    pub fn init_pool() -> Pool<AsyncPgConnection> {
        let database_url = get_test_db_url();
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                Pool::builder()
                    .max_size(20) // Maximum number of connections in the pool
                    .min_idle(Some(5)) // Minimum number of idle connections to maintain
                    .connection_timeout(std::time::Duration::from_secs(10)) // Connection timeout
                    .idle_timeout(Some(std::time::Duration::from_secs(300))) // Idle connection timeout
                    .max_lifetime(Some(std::time::Duration::from_secs(3600))) // Maximum connection lifetime
                    .build(manager)
                    .await
                    .expect("Failed to create test pool")
            })
        })
        .join()
        .expect("Thread panicked")
    }

    // Use a static pool for tests, initialized on first access
    static TEST_POOL: OnceLock<Pool<AsyncPgConnection>> = OnceLock::new();

    // Get pool reference
    pub fn get_pool() -> &'static Pool<AsyncPgConnection> {
        TEST_POOL.get_or_init(init_pool)
    }

    // Clean up pool between test runs
    pub async fn cleanup_pool() {
        if let Some(pool) = TEST_POOL.get() {
            // Wait for a bit to allow any in-progress operations to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Try to get a connection to ensure the pool is still working
            match pool.get().await {
                Ok(mut conn) => {
                    // First try to rollback any open transactions
                    let _ = conn.batch_execute("ROLLBACK").await;

                    // Then terminate any other connections that might be hanging
                    let _ = conn.batch_execute("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = current_database() AND state = 'idle in transaction' AND pid <> pg_backend_pid()").await;

                    // Wait a bit to ensure the rollback and termination completes
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Drop our connection
                    drop(conn);

                    // Wait for pool to stabilize
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
                Err(_) => {
                    // If we can't get a connection, the pool might be in a bad state
                    // Wait a bit longer to allow any remaining operations to complete
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    pub struct TestDb;

    impl TestDb {
        pub async fn new() -> Self {
            TestDb
        }

        pub async fn conn(&self) -> DbConn<'static> {
            // Get pool directly from the static reference
            let pool = get_pool();
            pool.get().await.expect("Failed to get connection")
        }
    }
}
