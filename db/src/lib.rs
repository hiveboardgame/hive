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

pub async fn get_pool(db_uri: &str) -> Result<DbPool, PoolError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_uri);
    Pool::builder().build(manager).await
}

pub async fn get_conn(
    pool: &DbPool,
) -> Result<PooledConnection<AsyncDieselConnectionManager<AsyncPgConnection>>, DieselError> {
    pool.get().await.map_err(|e| QueryBuilderError(e.into()))
}
