use anyhow::{Context, Result};
use db_lib::{get_conn, get_pool};
use dotenvy::dotenv;
use log::info;
use std::time::Duration;
use tokio::time::sleep;

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

pub fn log_progress(processed: usize, total: usize, operation: &str) {
    if processed % 1000 == 0 || processed == total {
        let percentage = if total > 0 {
            (processed as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        info!("{}: {}/{} ({}%)", operation, processed, total, percentage);
    }
}

pub fn log_operation_start(operation: &str) {
    info!("Starting {}...", operation);
}

pub fn log_operation_complete(operation: &str, processed: usize, errors: usize) {
    info!(
        "{} completed! Processed {} items with {} errors",
        operation, processed, errors
    );
}

pub async fn retry_operation<F, T>(operation: F, max_retries: usize, delay_ms: u64) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    info!(
                        "Attempt {} failed, retrying in {}ms: {}",
                        attempt,
                        delay_ms,
                        last_error.as_ref().unwrap()
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
