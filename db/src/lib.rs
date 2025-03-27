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
    use std::env;
    use std::sync::OnceLock;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::AsyncPgConnection;

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
                Pool::builder().build(manager).await.expect("Failed to create test pool")
            })
        }).join().expect("Thread panicked")
    }

    // Use a static pool for tests, initialized on first access
    static TEST_POOL: OnceLock<Pool<AsyncPgConnection>> = OnceLock::new();

    // Get pool reference
    pub fn get_pool() -> &'static Pool<AsyncPgConnection> {
        TEST_POOL.get_or_init(init_pool)
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
