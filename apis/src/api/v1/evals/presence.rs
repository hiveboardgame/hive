use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Idle workers ask for work every five seconds, so a minute without a claim means none is
/// running. The site is a single process, so this needs no shared storage.
const ONLINE_WITHIN_SECS: i64 = 60;

static LAST_CLAIM: AtomicI64 = AtomicI64::new(0);

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn worker_polled() {
    LAST_CLAIM.store(now_secs(), Ordering::Relaxed);
}

pub fn worker_seen_recently() -> bool {
    now_secs() - LAST_CLAIM.load(Ordering::Relaxed) <= ONLINE_WITHIN_SECS
}
