use super::{NotifyOutcome, Push};

/// iOS push backend, paused at the project level.
///
/// The mobile-side registration still emits `platform = 'apns'` rows in
/// `push_devices` from iOS builds — we keep the row so the day we resume
/// iOS push we don't lose user opt-ins — but server dispatch is intentionally
/// stubbed. See memory entry `ios_push_paused`.
///
/// Resuming iOS later means swapping the body for an APNs HTTP/2 client
/// (token auth via the `.p8` key from Apple Developer Program) and updating
/// `Notifiers::send` to wire `Some(ApnsNotifier::new(...))`.
pub struct ApnsNotifier {
    _private: (),
}

impl ApnsNotifier {
    pub async fn send(&self, _token: &str, _push: &Push) -> NotifyOutcome {
        NotifyOutcome::Failed("apns not implemented (iOS push paused)".into())
    }
}
