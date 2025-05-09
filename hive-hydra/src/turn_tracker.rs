use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::info;

// How long to keep the processed turns in memory (seconds) to avoid replaying them.
pub const TURN_RETENTION_TIME: Duration = Duration::from_secs(60);

#[async_trait]
pub trait TurnTracking {
    async fn tracked(&self, hash: u64) -> bool;
    async fn processing(&self, hash: u64);
    async fn processed(&self, hash: u64);
    async fn cleanup(&self);
}

#[derive(Clone)]
pub struct TurnTracker {
    processing_turns: Arc<Mutex<HashMap<u64, Instant>>>,
    processed_turns: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl TurnTracker {
    pub fn new() -> Self {
        TurnTracker {
            processing_turns: Arc::new(Mutex::new(HashMap::new())),
            processed_turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TurnTracking for TurnTracker {
    async fn tracked(&self, hash: u64) -> bool {
        let processing = self.processing_turns.lock().await;
        let processed = self.processed_turns.lock().await;
        processing.contains_key(&hash) || processed.contains_key(&hash)
    }

    async fn processing(&self, hash: u64) {
        self.processing_turns
            .lock()
            .await
            .insert(hash, Instant::now());
    }

    async fn processed(&self, hash: u64) {
        self.processing_turns.lock().await.remove(&hash);
        self.processed_turns
            .lock()
            .await
            .insert(hash, Instant::now());
    }

    async fn cleanup(&self) {
        let now = Instant::now();
        let mut processed = self.processed_turns.lock().await;
        processed.retain(|_, timestamp| now.duration_since(*timestamp) < TURN_RETENTION_TIME);

        info!("Processed_turns cleaned up");
    }
}
