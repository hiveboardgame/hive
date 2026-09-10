use anyhow::{Context, Result};
use db_lib::{get_conn, get_pool, DbConn};
use dotenvy::dotenv;

/// Leaks the pool so the connection can be `'static`, which is harmless in a
/// one-shot binary and saves threading a lifetime through every command.
pub async fn setup_database(database_url: Option<String>) -> Result<DbConn<'static>> {
    dotenv().ok();

    let database_url = database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .context("DATABASE_URL must be set, or --database-url provided")?;

    let pool = get_pool(&database_url)
        .await
        .context("could not build a connection pool")?;

    let static_pool = Box::leak(Box::new(pool));
    get_conn(static_pool)
        .await
        .context("could not take a connection from the pool")
}
