use std::time::Duration;

const POLL_EVERY: Duration = Duration::from_secs(1);

// Unset outside blue-green deployments, so every instance is active.
pub fn is_active_instance() -> bool {
    let Ok(active_file) = std::env::var("HIVE_ACTIVE_FILE") else {
        return true;
    };
    let Ok(site_addr) = std::env::var("LEPTOS_SITE_ADDR") else {
        return false;
    };
    std::fs::read_to_string(active_file).is_ok_and(|active| active.trim() == site_addr)
}

pub async fn wait_until_active() {
    while !is_active_instance() {
        actix_rt::time::sleep(POLL_EVERY).await;
    }
}
