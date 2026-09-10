use bb8::PooledConnection;
use diesel::result::{Error as DieselError, Error::QueryBuilderError};
use diesel_async::{
    pg::AsyncPgConnection,
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager, PoolError},
};

pub mod config;
pub mod db_error;
pub mod game_command;
pub mod helpers;
pub mod models;
pub mod schema;
pub mod tournaments;

#[cfg(test)]
extern crate self as db_lib;

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_support;

pub const DB_POOL_MAX_SIZE: u32 = 10;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn<'a> = PooledConnection<'a, AsyncDieselConnectionManager<AsyncPgConnection>>;

pub async fn get_pool(db_uri: &str) -> Result<DbPool, PoolError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_uri);
    Pool::builder()
        .max_size(DB_POOL_MAX_SIZE)
        .build(manager)
        .await
}

pub async fn get_conn(pool: &DbPool) -> Result<DbConn<'_>, DieselError> {
    pool.get().await.map_err(|e| QueryBuilderError(e.into()))
}
