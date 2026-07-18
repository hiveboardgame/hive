use crate::common::REALTIME_DISABLED_MSG;
use std::future::Future;
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
#[error("{REALTIME_DISABLED_MSG}")]
pub struct RealtimeDisabled;

#[derive(Debug)]
pub struct RealtimeGate {
    enabled: RwLock<bool>,
}

impl Default for RealtimeGate {
    fn default() -> Self {
        Self {
            enabled: RwLock::new(true),
        }
    }
}

impl RealtimeGate {
    pub async fn enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn with_realtime_admission<T, E, Fut>(
        &self,
        required: bool,
        mutation: Fut,
    ) -> anyhow::Result<T>
    where
        Fut: Future<Output = Result<T, E>>,
        E: Into<anyhow::Error>,
    {
        if !required {
            return mutation.await.map_err(Into::into);
        }
        // A queued writer closes admission immediately instead of leaving a
        // rejected request waiting while it holds a database connection.
        let enabled = self.enabled.try_read().map_err(|_| RealtimeDisabled)?;
        if !*enabled {
            return Err(RealtimeDisabled.into());
        }
        // The read guard is the permit; retain it until the mutation commits.
        let result = mutation.await.map_err(Into::into);
        drop(enabled);
        result
    }

    /// Serialize a complete state transition, including its broadcast.
    ///
    /// Disabling closes admission before draining already-admitted mutations.
    /// The caller runs this transition in an owned task so publication cannot
    /// be cancelled with the HTTP request that initiated it.
    pub(crate) async fn transition<Fut>(&self, enabled: bool, broadcast: Fut)
    where
        Fut: Future<Output = ()>,
    {
        let mut current = self.enabled.write().await;
        if *current == enabled {
            return;
        }
        *current = enabled;
        broadcast.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::{
        sync::{mpsc, oneshot},
        time::{timeout, Duration},
    };

    #[tokio::test]
    async fn operations_without_realtime_admission_bypass_disabled_gate() {
        let gate = RealtimeGate::default();
        gate.transition(false, async {}).await;
        assert!(gate
            .with_realtime_admission(false, async { Ok::<(), anyhow::Error>(()) })
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn disable_waits_for_admitted_mutation_and_rejects_new_admission() {
        let gate = RealtimeGate::default();
        let (release_tx, release_rx) = oneshot::channel();
        let mut admitted = Box::pin(gate.with_realtime_admission(true, async move {
            let _ = release_rx.await;
            Ok::<(), anyhow::Error>(())
        }));
        assert!(timeout(Duration::from_millis(20), admitted.as_mut())
            .await
            .is_err());

        let (broadcasted_tx, broadcasted_rx) = oneshot::channel();
        let mut disable = Box::pin(gate.transition(false, async move {
            let _ = broadcasted_tx.send(());
        }));
        assert!(timeout(Duration::from_millis(20), disable.as_mut())
            .await
            .is_err());

        let rejected = gate
            .with_realtime_admission(true, async { Ok::<(), anyhow::Error>(()) })
            .await
            .unwrap_err();
        assert_eq!(rejected.to_string(), REALTIME_DISABLED_MSG);

        let _ = release_tx.send(());
        assert!(admitted.await.is_ok());
        disable.await;
        assert!(broadcasted_rx.await.is_ok());
        assert!(!gate.enabled().await);
    }

    #[tokio::test]
    async fn concurrent_transitions_broadcast_in_transition_order() {
        let gate = Arc::new(RealtimeGate::default());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (release_tx, release_rx) = oneshot::channel();

        let first_gate = gate.clone();
        let first_tx = tx.clone();
        let first = tokio::spawn(async move {
            first_gate
                .transition(false, async move {
                    first_tx.send(false).unwrap();
                    let _ = release_rx.await;
                })
                .await;
        });
        assert_eq!(rx.recv().await, Some(false));

        let second_gate = gate.clone();
        let second = tokio::spawn(async move {
            second_gate
                .transition(true, async move {
                    tx.send(true).unwrap();
                })
                .await;
        });

        let _ = release_tx.send(());
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(rx.recv().await, Some(true));
        assert!(gate.enabled().await);
    }
}
