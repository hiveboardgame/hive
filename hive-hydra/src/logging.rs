use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    Registry,
    EnvFilter,
};

pub fn setup_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // File appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "hive-hydra.log");

    // Create a custom time formatter
    let timer = UtcTime::new(
        time::format_description::parse(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
        )
        .expect("Invalid time format"),
    );

    // Create env filter from RUST_LOG environment variable (defaults to info if not set)
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Console layer with colored output and local time
    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_line_number(false)
        .with_file(false)
        .with_level(true)
        .with_timer(timer.clone());

    // File layer with more detailed output
    let file_layer = fmt::layer()
        .with_target(false)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(file_appender)
        .with_timer(timer);

    // Combine layers with the env filter
    let subscriber = Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer);

    // Set as global default
    tracing::subscriber::set_global_default(subscriber)?;

    // Log initial message to verify setup
    info!("Logging system initialized");

    Ok(())
}
