use super::{Chat, TimeoutControls};
use crate::common::{ClientRequest, SubscriptionAttempt, SubscriptionError};
use leptos::prelude::*;
use shared_types::ConversationKey;
use std::time::Duration;
use web_time::Instant;

pub(super) const ACK_TIMEOUT: Duration = Duration::from_secs(8);
const TIMEOUT_RETRY_DELAY: Duration = Duration::from_secs(1);
const SEND_RETRY_DELAY: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionIssue {
    TimedOut,
    AccessDenied,
    RateLimited,
    Unavailable,
}

impl SubscriptionIssue {
    fn allows_early_retry(&self) -> bool {
        matches!(self, Self::TimedOut | Self::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandshakeState {
    Queued,
    Pending {
        request_id: u64,
        deadline: Instant,
    },
    Ready,
    Failed {
        issue: SubscriptionIssue,
        retry_at: Option<Instant>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubscriptionSlot {
    key: ConversationKey,
    session_epoch: u64,
    owner_id: u64,
    handshake: HandshakeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Pending,
    Ready,
    Retryable {
        issue: SubscriptionIssue,
        can_retry_now: bool,
    },
    Failed {
        issue: SubscriptionIssue,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubscriptionOwner {
    key: ConversationKey,
    owner_id: u64,
}

#[derive(Clone, Debug, Default)]
struct CoordinatorState {
    slot: Option<SubscriptionSlot>,
    next_owner_id: u64,
    next_request_id: u64,
    connection_open: bool,
    identity_resolved: bool,
}

impl CoordinatorState {
    fn prerequisites_ready(&self) -> bool {
        self.connection_open && self.identity_resolved
    }

    fn allocate_owner_id(&mut self) -> u64 {
        self.next_owner_id = self
            .next_owner_id
            .checked_add(1)
            .expect("chat subscription owner IDs exhausted");
        self.next_owner_id
    }

    fn allocate_request_id(&mut self) -> u64 {
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .expect("chat subscription request IDs exhausted");
        self.next_request_id
    }

    fn acquire(
        &mut self,
        key: ConversationKey,
        session_epoch: u64,
    ) -> (SubscriptionOwner, Option<ConversationKey>) {
        let replaced_key = self
            .connection_open
            .then(|| self.slot.as_ref().map(|slot| slot.key.clone()))
            .flatten();
        let owner_id = self.allocate_owner_id();
        self.slot = Some(SubscriptionSlot {
            key: key.clone(),
            session_epoch,
            owner_id,
            handshake: HandshakeState::Queued,
        });
        (SubscriptionOwner { key, owner_id }, replaced_key)
    }

    fn release(&mut self, owner: &SubscriptionOwner) -> Option<ConversationKey> {
        let slot = self.slot.as_ref()?;
        if slot.key != owner.key || slot.owner_id != owner.owner_id {
            return None;
        }

        let key = slot.key.clone();
        self.slot = None;
        self.connection_open.then_some(key)
    }

    fn update_prerequisites(
        &mut self,
        connection_open: bool,
        identity_resolved: bool,
        session_epoch: u64,
    ) {
        self.connection_open = connection_open;
        self.identity_resolved = identity_resolved;

        if !connection_open {
            if let Some(slot) = self.slot.as_mut() {
                slot.handshake = HandshakeState::Queued;
            }
            return;
        }
        if !identity_resolved {
            return;
        }

        if let Some(slot) = self
            .slot
            .as_mut()
            .filter(|slot| slot.session_epoch != session_epoch)
        {
            slot.session_epoch = session_epoch;
            slot.handshake = HandshakeState::Queued;
        }
    }

    fn begin_attempt(&mut self, now: Instant) -> Option<SubscriptionAttempt> {
        if !self.prerequisites_ready()
            || !self
                .slot
                .as_ref()
                .is_some_and(|slot| matches!(&slot.handshake, HandshakeState::Queued))
        {
            return None;
        }

        let request_id = self.allocate_request_id();
        let slot = self.slot.as_mut()?;
        slot.handshake = HandshakeState::Pending {
            request_id,
            deadline: now + ACK_TIMEOUT,
        };
        Some(SubscriptionAttempt {
            key: slot.key.clone(),
            session_epoch: slot.session_epoch,
            request_id,
        })
    }

    fn retry(&mut self, key: &ConversationKey, session_epoch: u64, now: Instant) -> bool {
        if !self.prerequisites_ready() {
            return false;
        }
        let Some(slot) = self.current_slot_mut(key, session_epoch) else {
            return false;
        };
        let (issue, retry_at) = match &slot.handshake {
            HandshakeState::Failed {
                issue,
                retry_at: Some(retry_at),
                ..
            } => (issue, *retry_at),
            HandshakeState::Queued
            | HandshakeState::Pending { .. }
            | HandshakeState::Ready
            | HandshakeState::Failed { retry_at: None, .. } => return false,
        };
        if retry_at > now && !issue.allows_early_retry() {
            return false;
        }
        slot.handshake = HandshakeState::Queued;
        true
    }

    fn confirm(&mut self, key: &ConversationKey, session_epoch: u64, request_id: u64) -> bool {
        self.resolve_attempt(key, session_epoch, request_id, HandshakeState::Ready)
    }

    fn fail(
        &mut self,
        key: &ConversationKey,
        session_epoch: u64,
        request_id: u64,
        error: SubscriptionError,
        now: Instant,
    ) -> bool {
        let (issue, retry_at) = match error {
            SubscriptionError::RateLimited { retry_after } => (
                SubscriptionIssue::RateLimited,
                Some(now + retry_after.max(Duration::from_millis(1))),
            ),
            SubscriptionError::AccessDenied => (SubscriptionIssue::AccessDenied, None),
            SubscriptionError::Unavailable => {
                (SubscriptionIssue::Unavailable, Some(now + SEND_RETRY_DELAY))
            }
        };
        self.resolve_attempt(
            key,
            session_epoch,
            request_id,
            HandshakeState::Failed { issue, retry_at },
        )
    }

    fn fail_send(&mut self, request: &SubscriptionAttempt, now: Instant) -> bool {
        self.resolve_attempt(
            &request.key,
            request.session_epoch,
            request.request_id,
            HandshakeState::Failed {
                issue: SubscriptionIssue::Unavailable,
                retry_at: Some(now + SEND_RETRY_DELAY),
            },
        )
    }

    fn resolve_attempt(
        &mut self,
        key: &ConversationKey,
        session_epoch: u64,
        request_id: u64,
        resolved_state: HandshakeState,
    ) -> bool {
        let Some(slot) = self.current_slot_mut(key, session_epoch) else {
            return false;
        };
        if !matches!(
            &slot.handshake,
            HandshakeState::Pending {
                request_id: pending_request_id,
                ..
            } if *pending_request_id == request_id
        ) {
            return false;
        }
        slot.handshake = resolved_state;
        true
    }

    fn current_slot(&self, key: &ConversationKey, session_epoch: u64) -> Option<&SubscriptionSlot> {
        self.slot
            .as_ref()
            .filter(|slot| &slot.key == key && slot.session_epoch == session_epoch)
    }

    fn current_slot_mut(
        &mut self,
        key: &ConversationKey,
        session_epoch: u64,
    ) -> Option<&mut SubscriptionSlot> {
        self.slot
            .as_mut()
            .filter(|slot| &slot.key == key && slot.session_epoch == session_epoch)
    }

    fn advance(&mut self, now: Instant) {
        if !self.prerequisites_ready() {
            return;
        }

        let Some(slot) = self.slot.as_mut() else {
            return;
        };
        match &slot.handshake {
            HandshakeState::Pending { deadline, .. } if *deadline <= now => {
                slot.handshake = HandshakeState::Failed {
                    issue: SubscriptionIssue::TimedOut,
                    retry_at: Some(now + TIMEOUT_RETRY_DELAY),
                };
            }
            HandshakeState::Failed {
                retry_at: Some(retry_at),
                ..
            } if *retry_at <= now => {
                slot.handshake = HandshakeState::Queued;
            }
            HandshakeState::Queued
            | HandshakeState::Pending { .. }
            | HandshakeState::Ready
            | HandshakeState::Failed { .. } => {}
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        if !self.prerequisites_ready() {
            return None;
        }
        self.slot.as_ref().and_then(|slot| match &slot.handshake {
            HandshakeState::Pending { deadline, .. } => Some(*deadline),
            HandshakeState::Failed {
                retry_at: Some(retry_at),
                ..
            } => Some(*retry_at),
            HandshakeState::Queued
            | HandshakeState::Ready
            | HandshakeState::Failed { retry_at: None, .. } => None,
        })
    }

    fn view(&self, key: &ConversationKey, session_epoch: u64, now: Instant) -> SubscriptionStatus {
        let Some(slot) = self.current_slot(key, session_epoch) else {
            return SubscriptionStatus::Pending;
        };
        match &slot.handshake {
            HandshakeState::Queued | HandshakeState::Pending { .. } => SubscriptionStatus::Pending,
            HandshakeState::Ready => SubscriptionStatus::Ready,
            HandshakeState::Failed {
                issue,
                retry_at: Some(retry_at),
                ..
            } => SubscriptionStatus::Retryable {
                issue: issue.clone(),
                can_retry_now: self.prerequisites_ready()
                    && (*retry_at <= now || issue.allows_early_retry()),
            },
            HandshakeState::Failed {
                issue,
                retry_at: None,
                ..
            } => SubscriptionStatus::Failed {
                issue: issue.clone(),
            },
        }
    }
}

#[derive(Copy, Clone)]
pub(super) struct SubscriptionCoordinator {
    state: RwSignal<CoordinatorState>,
    retry_timer: StoredValue<Option<TimeoutControls<()>>>,
}

impl SubscriptionCoordinator {
    pub(super) fn new() -> Self {
        Self {
            state: RwSignal::new(CoordinatorState::default()),
            retry_timer: StoredValue::new(None),
        }
    }

    pub(super) fn install_retry_timer(&self, timer: TimeoutControls<()>) {
        self.retry_timer.set_value(Some(timer));
    }
}

fn requires_subscription(key: &ConversationKey) -> bool {
    matches!(
        key,
        ConversationKey::Tournament(_)
            | ConversationKey::Game {
                thread: shared_types::GameThread::Spectators,
                ..
            }
    )
}

impl Chat {
    pub(crate) fn use_subscription(
        &self,
        channel_key: ConversationKey,
    ) -> Memo<SubscriptionStatus> {
        let chat = *self;
        let owner = requires_subscription(&channel_key).then(|| {
            chat.acquire_subscription(channel_key.clone(), chat.session_epoch.get_untracked())
        });

        on_cleanup(move || {
            if let Some(owner) = owner {
                chat.release_subscription(owner);
            }
        });

        Memo::new(move |_| {
            if requires_subscription(&channel_key) {
                chat.subscription_status(&channel_key, chat.session_epoch())
            } else {
                SubscriptionStatus::Ready
            }
        })
    }

    pub(crate) fn subscription_ready_for_history(
        &self,
        key: &ConversationKey,
        session_epoch: u64,
    ) -> bool {
        !requires_subscription(key)
            || matches!(
                self.subscription_status(key, session_epoch),
                SubscriptionStatus::Ready
            )
    }

    fn acquire_subscription(&self, key: ConversationKey, session_epoch: u64) -> SubscriptionOwner {
        let now = Instant::now();
        let (owner, replaced_key) = self
            .subscriptions
            .state
            .try_update(|state| state.acquire(key, session_epoch))
            .expect("chat subscription coordinator was disposed");
        if let Some(key) = replaced_key {
            self.websocket.with_value(|websocket| {
                websocket.send(&ClientRequest::ChatUnsubscribe(key));
            });
        }
        self.sync_subscription(now);
        owner
    }

    fn release_subscription(&self, owner: SubscriptionOwner) {
        let key = self
            .subscriptions
            .state
            .try_update(|state| state.release(&owner))
            .expect("chat subscription coordinator was disposed");
        if let Some(key) = key {
            self.websocket.with_value(|websocket| {
                websocket.send(&ClientRequest::ChatUnsubscribe(key));
            });
        }
        self.sync_subscription(Instant::now());
    }

    pub(super) fn update_subscription_prerequisites(
        &self,
        connection_open: bool,
        identity_resolved: bool,
    ) {
        let now = Instant::now();
        let session_epoch = self.session_epoch.get_untracked();
        self.subscriptions.state.update(|state| {
            state.update_prerequisites(connection_open, identity_resolved, session_epoch);
        });
        self.sync_subscription(now);
    }

    pub(crate) fn retry_subscription(&self, key: ConversationKey, session_epoch: u64) -> bool {
        if session_epoch != self.session_epoch.get_untracked() {
            return false;
        }
        let now = Instant::now();
        if !self
            .subscriptions
            .state
            .try_update(|state| state.retry(&key, session_epoch, now))
            .expect("chat subscription coordinator was disposed")
        {
            return false;
        }
        self.sync_subscription(now);
        true
    }

    fn sync_subscription(&self, now: Instant) {
        let request = self
            .subscriptions
            .state
            .try_update(|state| state.begin_attempt(now))
            .expect("chat subscription coordinator was disposed");
        if let Some(request) = request {
            if !self.websocket.with_value(|websocket| {
                websocket.send(&ClientRequest::ChatSubscribe(request.clone()))
            }) {
                self.subscriptions.state.update(|state| {
                    state.fail_send(&request, now);
                });
            }
        }
        self.schedule_subscription_timer(now);
    }

    pub(crate) fn confirm_subscription(&self, attempt: SubscriptionAttempt) {
        if attempt.session_epoch != self.session_epoch.get_untracked() {
            return;
        }
        if self
            .subscriptions
            .state
            .try_update(|state| {
                state.confirm(&attempt.key, attempt.session_epoch, attempt.request_id)
            })
            .expect("chat subscription coordinator was disposed")
        {
            self.sync_subscription(Instant::now());
        }
    }

    fn subscription_status(&self, key: &ConversationKey, session_epoch: u64) -> SubscriptionStatus {
        self.subscriptions
            .state
            .with(|state| state.view(key, session_epoch, Instant::now()))
    }

    pub(crate) fn fail_subscription(&self, attempt: SubscriptionAttempt, error: SubscriptionError) {
        if attempt.session_epoch != self.session_epoch.get_untracked() {
            return;
        }
        let now = Instant::now();
        if self
            .subscriptions
            .state
            .try_update(|state| {
                state.fail(
                    &attempt.key,
                    attempt.session_epoch,
                    attempt.request_id,
                    error,
                    now,
                )
            })
            .expect("chat subscription coordinator was disposed")
        {
            self.sync_subscription(now);
        }
    }

    fn schedule_subscription_timer(&self, now: Instant) {
        let next_deadline = self
            .subscriptions
            .state
            .with_untracked(CoordinatorState::next_deadline);
        let Some(timer) = self.subscriptions.retry_timer.get_value() else {
            return;
        };
        if let Some(next_deadline) = next_deadline {
            timer.schedule(
                next_deadline
                    .saturating_duration_since(now)
                    .max(Duration::from_millis(1)),
                (),
            );
        } else {
            timer.stop();
        }
    }

    pub(super) fn advance_subscription(&self) {
        let now = Instant::now();
        self.subscriptions.state.update(|state| {
            state.advance(now);
        });
        self.sync_subscription(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::GameId;

    fn key(name: &str) -> ConversationKey {
        ConversationKey::game_spectators(&GameId(name.to_string()))
    }

    fn ready_state() -> CoordinatorState {
        CoordinatorState {
            connection_open: true,
            identity_resolved: true,
            ..CoordinatorState::default()
        }
    }

    #[test]
    fn same_key_handoff_ignores_old_owner_cleanup() {
        let now = Instant::now();
        let channel = key("shared");
        let mut state = ready_state();

        let (old_owner, replaced) = state.acquire(channel.clone(), 3);
        assert_eq!(replaced, None);
        _ = state.begin_attempt(now).unwrap();
        let (replacement_owner, replaced) = state.acquire(channel.clone(), 3);

        assert_ne!(old_owner, replacement_owner);
        assert_eq!(replaced, Some(channel.clone()));
        assert_eq!(state.release(&old_owner), None);
        assert_eq!(
            state.slot.as_ref().map(|slot| slot.owner_id),
            Some(replacement_owner.owner_id),
        );
        assert!(state.begin_attempt(now).is_some());
        assert_eq!(state.release(&replacement_owner), Some(channel));
        assert!(state.slot.is_none());
    }

    #[test]
    fn replacement_is_permanent_and_a_b_a_rejects_stale_work() {
        let now = Instant::now();
        let a = key("a");
        let b = key("b");
        let mut state = ready_state();

        let (old_a_lease, _) = state.acquire(a.clone(), 7);
        let old_a = state.begin_attempt(now).unwrap();
        let (b_lease, replaced) = state.acquire(b.clone(), 7);
        assert_eq!(replaced, Some(a.clone()));
        let only_b = state.begin_attempt(now).unwrap();
        let (current_a_lease, replaced) = state.acquire(a.clone(), 7);
        assert_eq!(replaced, Some(b));
        let current_a = state.begin_attempt(now).unwrap();

        assert_eq!(
            [old_a.request_id, only_b.request_id, current_a.request_id],
            [1, 2, 3],
        );
        assert_eq!(state.release(&old_a_lease), None);
        assert_eq!(state.release(&b_lease), None);
        assert_eq!(state.slot.as_ref().map(|slot| &slot.key), Some(&a));
        assert!(!state.confirm(&a, 7, old_a.request_id));
        assert!(!state.fail(
            &a,
            7,
            old_a.request_id,
            SubscriptionError::AccessDenied,
            now,
        ));
        assert!(!state.confirm(&a, 6, current_a.request_id));
        assert!(!state.confirm(&a, 7, current_a.request_id + 1));
        assert!(state.confirm(&a, 7, current_a.request_id));
        assert_eq!(state.view(&a, 7, now), SubscriptionStatus::Ready);
        assert_eq!(state.release(&current_a_lease), Some(a));
    }

    #[test]
    fn disconnect_retains_owner_blocks_advancement_and_reconnects_once() {
        let now = Instant::now();
        let channel = key("reconnect");
        let mut state = ready_state();
        let (owner, _) = state.acquire(channel.clone(), 9);
        assert_eq!(state.begin_attempt(now).unwrap().request_id, 1);

        state.update_prerequisites(false, true, 9);
        assert!(state.begin_attempt(now).is_none());
        assert!(matches!(
            state.slot.as_ref().map(|slot| &slot.handshake),
            Some(HandshakeState::Queued)
        ));
        assert!(!state.retry(&channel, 9, now));
        state.advance(now + Duration::from_secs(30));
        assert!(state.begin_attempt(now).is_none());
        assert_eq!(state.next_deadline(), None);

        let mut released_while_disconnected = state.clone();
        assert_eq!(released_while_disconnected.release(&owner), None);
        assert!(released_while_disconnected.slot.is_none());

        state.update_prerequisites(true, true, 9);
        assert_eq!(state.begin_attempt(now).unwrap().request_id, 2);
        state.update_prerequisites(true, true, 9);
        assert!(state.begin_attempt(now).is_none());
        assert_eq!(state.next_request_id, 2);
    }

    #[test]
    fn identity_gate_blocks_requests_and_retries_until_resolved() {
        let now = Instant::now();
        let channel = key("pending-identity");
        let mut state = CoordinatorState {
            connection_open: true,
            ..CoordinatorState::default()
        };
        let _ = state.acquire(channel.clone(), 14);
        assert!(state.begin_attempt(now).is_none());
        assert_eq!(state.next_request_id, 0);

        state.update_prerequisites(true, true, 14);
        let request = state.begin_attempt(now).unwrap();
        assert!(state.fail(
            &channel,
            14,
            request.request_id,
            SubscriptionError::Unavailable,
            now,
        ));

        let retry_time = now + Duration::from_secs(1);
        state.update_prerequisites(true, false, 14);
        assert!(!state.retry(&channel, 14, retry_time));
        state.advance(retry_time);
        assert_eq!(state.next_deadline(), None);
        assert_eq!(
            state.view(&channel, 14, retry_time),
            SubscriptionStatus::Retryable {
                issue: SubscriptionIssue::Unavailable,
                can_retry_now: false,
            },
        );

        state.update_prerequisites(true, true, 14);
        assert!(state.retry(&channel, 14, retry_time));
        let retry = state.begin_attempt(retry_time).unwrap();
        assert_eq!(retry.request_id, 2);

        let timeout = retry_time + ACK_TIMEOUT;
        state.advance(timeout);
        assert_eq!(
            state.view(&channel, 14, timeout),
            SubscriptionStatus::Retryable {
                issue: SubscriptionIssue::TimedOut,
                can_retry_now: true,
            },
        );

        let retry_time = timeout + TIMEOUT_RETRY_DELAY;
        state.advance(retry_time);
        let request = state.begin_attempt(retry_time).unwrap();
        assert_eq!(request.request_id, 3);
        assert!(state.fail(
            &channel,
            14,
            request.request_id,
            SubscriptionError::AccessDenied,
            retry_time,
        ));
        assert_eq!(
            state.view(&channel, 14, retry_time),
            SubscriptionStatus::Failed {
                issue: SubscriptionIssue::AccessDenied,
            },
        );
        assert_eq!(state.next_deadline(), None);
    }
}
