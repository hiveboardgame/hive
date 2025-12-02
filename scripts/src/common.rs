use anyhow::{Context, Result};
use db_lib::{get_conn, get_pool};
use dotenvy::dotenv;
use log::info;

const PROGRESS_LOG_INTERVAL: usize = 1000;

pub async fn setup_database(database_url: Option<String>) -> Result<db_lib::DbConn<'static>> {
    dotenv().ok();

    let database_url = database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .context("DATABASE_URL environment variable must be set or --database-url provided")?;

    let pool = get_pool(&database_url)
        .await
        .context("Failed to create database connection pool")?;

    let static_pool = Box::leak(Box::new(pool));
    let conn = get_conn(static_pool)
        .await
        .context("Failed to get database connection from pool")?;
    Ok(conn)
}

pub const TEST_USER_USERNAME_PATTERN: &str = "testuser%";
pub const TEST_USER_EMAIL_PATTERN: &str = "test%@example.com";

pub fn log_progress(processed: usize, total: usize, operation: &str) {
    if processed.is_multiple_of(PROGRESS_LOG_INTERVAL) || processed == total {
        let percentage = if total > 0 {
            (processed as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        info!("{operation}: {processed}/{total} ({percentage}%)");
    }
}

pub fn log_operation_start(operation: &str) {
    info!("Starting {operation}...");
}

pub fn log_operation_complete(operation: &str, processed: usize, errors: usize) {
    info!(
        "{operation} completed! Processed {processed} items with {errors} errors"
    );
}

