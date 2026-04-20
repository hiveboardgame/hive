use std::sync::atomic::{AtomicU64, Ordering};

static CHAT_PERSIST_ATTEMPTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CHAT_PERSIST_SUCCESSES_TOTAL: AtomicU64 = AtomicU64::new(0);
static CHAT_PERSIST_FAILURES_TOTAL: AtomicU64 = AtomicU64::new(0);
static CHAT_MESSAGE_NORMALIZATIONS_TOTAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct ChatMetricsSnapshot {
    pub persist_attempts_total: u64,
    pub persist_successes_total: u64,
    pub persist_failures_total: u64,
    pub message_normalizations_total: u64,
}

pub fn record_persist_attempt() {
    CHAT_PERSIST_ATTEMPTS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub fn record_persist_success() {
    CHAT_PERSIST_SUCCESSES_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub fn record_persist_failure() {
    CHAT_PERSIST_FAILURES_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub fn record_message_normalization() {
    CHAT_MESSAGE_NORMALIZATIONS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub fn snapshot() -> ChatMetricsSnapshot {
    ChatMetricsSnapshot {
        persist_attempts_total: CHAT_PERSIST_ATTEMPTS_TOTAL.load(Ordering::Relaxed),
        persist_successes_total: CHAT_PERSIST_SUCCESSES_TOTAL.load(Ordering::Relaxed),
        persist_failures_total: CHAT_PERSIST_FAILURES_TOTAL.load(Ordering::Relaxed),
        message_normalizations_total: CHAT_MESSAGE_NORMALIZATIONS_TOTAL.load(Ordering::Relaxed),
    }
}
