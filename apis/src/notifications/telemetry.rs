use std::sync::atomic::{AtomicU64, Ordering};

use shared_types::PushMetrics;

#[derive(Debug, Default)]
pub struct PushTelemetry {
    received: AtomicU64,
    dropped_queue_full: AtomicU64,
    suppressed_prefs: AtomicU64,
    prefs_db_error: AtomicU64,
    ack_eligible: AtomicU64,
    ack_suppressed: AtomicU64,
    ack_fired: AtomicU64,
    test_pushes: AtomicU64,
    no_device: AtomicU64,
    device_db_error: AtomicU64,
    delivered: AtomicU64,
    retryable: AtomicU64,
    token_dead: AtomicU64,
    failed: AtomicU64,
    retry_delivered: AtomicU64,
    retry_gave_up: AtomicU64,
}

macro_rules! bump {
    ($name:ident) => {
        pub fn $name(&self) {
            self.$name.fetch_add(1, Ordering::Relaxed);
        }
    };
}

impl PushTelemetry {
    bump!(received);
    bump!(dropped_queue_full);
    bump!(suppressed_prefs);
    bump!(prefs_db_error);
    bump!(ack_eligible);
    bump!(ack_suppressed);
    bump!(ack_fired);
    bump!(test_pushes);
    bump!(no_device);
    bump!(device_db_error);
    bump!(delivered);
    bump!(retryable);
    bump!(token_dead);
    bump!(failed);
    bump!(retry_delivered);
    bump!(retry_gave_up);

    pub fn snapshot(&self) -> PushMetrics {
        let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
        PushMetrics {
            received: g(&self.received),
            dropped_queue_full: g(&self.dropped_queue_full),
            suppressed_prefs: g(&self.suppressed_prefs),
            prefs_db_error: g(&self.prefs_db_error),
            ack_eligible: g(&self.ack_eligible),
            ack_suppressed: g(&self.ack_suppressed),
            ack_fired: g(&self.ack_fired),
            test_pushes: g(&self.test_pushes),
            no_device: g(&self.no_device),
            device_db_error: g(&self.device_db_error),
            delivered: g(&self.delivered),
            retryable: g(&self.retryable),
            token_dead: g(&self.token_dead),
            failed: g(&self.failed),
            retry_delivered: g(&self.retry_delivered),
            retry_gave_up: g(&self.retry_gave_up),
        }
    }
}
