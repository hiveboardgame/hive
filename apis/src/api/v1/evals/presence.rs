use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// Idle workers ask for work every five seconds and busy ones report progress as often, so a
/// minute without either means the worker is gone. The site is a single process, so this
/// needs no shared storage.
const ONLINE_WITHIN_SECS: i64 = 60;

static LAST_SEEN: LazyLock<Mutex<HashMap<String, i64>>> = LazyLock::new(Default::default);

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn worker_seen(worker: &str) {
    let now = now_secs();
    if let Ok(mut seen) = LAST_SEEN.lock() {
        seen.retain(|_, last| now - *last <= ONLINE_WITHIN_SECS);
        seen.insert(worker.to_string(), now);
    }
}

pub fn workers_online() -> usize {
    let now = now_secs();
    LAST_SEEN.lock().map_or(0, |seen| {
        seen.values()
            .filter(|last| now - **last <= ONLINE_WITHIN_SECS)
            .count()
    })
}
