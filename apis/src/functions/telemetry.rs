use leptos::prelude::*;
use shared_types::{TelemetryRange, TelemetryRow};

/// Resolve the CSV path used by both the snapshot writer and the admin
/// dashboard reader. In release the default is `./ws_metrics.csv`; in debug
/// it's unset. `WS_METRICS_LOG_FILE` overrides; an empty value disables CSV.
pub fn resolve_csv_path() -> Option<String> {
    let in_release = !cfg!(debug_assertions);
    match std::env::var("WS_METRICS_LOG_FILE") {
        Ok(s) if s.is_empty() => None,
        Ok(s) => Some(s),
        Err(_) if in_release => Some("./ws_metrics.csv".to_string()),
        Err(_) => None,
    }
}

#[server(client = crate::client::ApiClient)]
pub async fn read_telemetry(range: TelemetryRange) -> Result<Vec<TelemetryRow>, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::get_conn;
    use std::{
        io::{BufRead, BufReader},
        time::{SystemTime, UNIX_EPOCH},
    };

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;

    let path = resolve_csv_path().ok_or_else(|| {
        ServerFnError::new(
            "Telemetry CSV is disabled (WS_METRICS_LOG_FILE is empty) or \
             not configured for this build. In debug builds set \
             WS_METRICS_LOG_FILE to a path to enable.",
        )
    })?;

    let file = std::fs::File::open(&path)
        .map_err(|e| ServerFnError::new(format!("Could not open {path}: {e}")))?;
    let reader = BufReader::new(file);

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cutoff = range.cutoff_secs(now_secs);

    let mut rows = Vec::new();
    for line in reader.lines().map_while(Result::ok) {
        if line.is_empty() || line.starts_with("timestamp,") {
            continue;
        }
        if let Some(row) = TelemetryRow::from_csv_line(&line) {
            if row.timestamp >= cutoff {
                rows.push(row);
            }
        }
    }
    Ok(rows)
}
