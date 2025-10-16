use db_lib::{get_conn, get_pool};
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;
use dotenvy::dotenv;

#[derive(Debug, Clone)]
pub struct Config {
    pub progress_interval: usize,
    pub csv_buffer_size: usize,
    pub max_retries: usize,
    pub temp_file_prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            progress_interval: 1000,
            csv_buffer_size: 8192,
            max_retries: 3,
            temp_file_prefix: "hive_script_".to_string(),
        }
    }
}

pub async fn setup_database(
    database_url: Option<String>,
) -> Result<db_lib::DbConn<'static>, Box<dyn Error>> {
    dotenv().ok();
    
    let database_url = database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .expect("DATABASE_URL environment variable must be set or --database-url provided");
    
    let pool = get_pool(&database_url).await?;
    
    // Use Box::leak to create a static reference to the pool
    let static_pool = Box::leak(Box::new(pool));
    let conn = get_conn(static_pool).await?;
    Ok(conn)
}

pub fn log_progress(processed: usize, total: usize, operation: &str) {
    if processed % 1000 == 0 || processed == total {
        let percentage = if total > 0 {
            (processed as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        println!("{}: {}/{} ({}%)", operation, processed, total, percentage);
    }
}

pub fn log_info(message: &str) {
    println!("Info: {}", message);
}

pub fn log_operation_start(operation: &str) {
    println!("Starting {}...", operation);
}

pub fn log_operation_complete(operation: &str, processed: usize, errors: usize) {
    println!("{} completed! Processed {} items with {} errors", operation, processed, errors);
}

pub fn log_warning(message: &str) {
    println!("Warning: {}", message);
}

pub fn log_error(message: &str) {
    eprintln!("Error: {}", message);
}

pub async fn retry_operation<F, T, E>(
    operation: F,
    max_retries: usize,
    delay_ms: u64,
) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    
    for attempt in 1..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    log_warning(&format!("Attempt {} failed, retrying in {}ms: {}", attempt, delay_ms, last_error.as_ref().unwrap()));
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

pub fn create_safe_csv_writer(filename: &str) -> Result<tempfile::NamedTempFile, Box<dyn Error>> {
    let temp_file = tempfile::NamedTempFile::new()?;
    log_info(&format!("Created temporary file for {}", filename));
    Ok(temp_file)
}

pub fn persist_csv_file(temp_file: tempfile::NamedTempFile, filename: &str) -> Result<(), Box<dyn Error>> {
    temp_file.persist(filename)?;
    log_info(&format!("Successfully wrote {}", filename));
    Ok(())
}
