mod conversation;
#[cfg(test)]
mod facade_tests;
mod history;
mod inbox;
mod outgoing;
mod preferences;
mod subscriptions;
mod unread;

use conversation::ConversationRegistry;
pub use conversation::{ConversationHandle, UnreadDisplay};
pub(crate) use history::{InitialHistoryStatus, OlderHistoryStatus};
pub use outgoing::{OutgoingChat, OutgoingState, SendIssue};
use preferences::ChatPreferences;
use subscriptions::SubscriptionCoordinator;
pub use subscriptions::{SubscriptionIssue, SubscriptionStatus};
use unread::{ReadReceiptCoordinator, UnreadSummary};

use super::{
    auth_context::{AuthContext, AuthIdentity},
    websocket::{ConnectionReadyState, WebsocketContext},
};
use leptos::prelude::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use shared_types::{ChatInboxSnapshot, ConversationKey, GameId, TournamentId};
use std::{collections::HashMap, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone, Debug)]
struct VisibleChannelOwner {
    id: u64,
    key: ConversationKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogActivity {
    pub(crate) key: ConversationKey,
    pub(crate) message_id: i64,
}

#[derive(Clone)]
struct TimeoutControls<Arg> {
    delay_ms: RwSignal<f64>,
    is_pending: Signal<bool>,
    start: Arc<dyn Fn(Arg) + Send + Sync>,
    stop: Arc<dyn Fn() + Send + Sync>,
}

impl<Arg> TimeoutControls<Arg> {
    fn new(
        delay_ms: RwSignal<f64>,
        is_pending: Signal<bool>,
        start: impl Fn(Arg) + Send + Sync + 'static,
        stop: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            delay_ms,
            is_pending,
            start: Arc::new(start),
            stop: Arc::new(stop),
        }
    }

    fn schedule(&self, delay: Duration, arg: Arg) {
        (self.stop)();
        self.delay_ms.set(delay.as_secs_f64() * 1_000.0);
        (self.start)(arg);
    }

    fn stop(&self) {
        (self.stop)();
    }
}

fn bump_generation(generation: RwSignal<u64>) -> u64 {
    generation
        .try_update(|generation| {
            *generation = generation.saturating_add(1);
            *generation
        })
        .unwrap_or_default()
}

#[cfg(test)]
fn test_websocket() -> WebsocketContext {
    use crate::common::ServerResult;

    WebsocketContext::new(
        Signal::derive(|| None::<ServerResult>),
        Arc::new(|_| true),
        Signal::derive(|| ConnectionReadyState::Open),
        Arc::new(|| {}),
        Arc::new(|| {}),
        Arc::new(|| {}),
    )
}

#[derive(Copy, Clone)]
pub struct Chat {
    conversations: StoredValue<ConversationRegistry>,
    unread: StoredValue<HashMap<ConversationKey, unread::ConversationUnread>>,
    read_receipts: StoredValue<ReadReceiptCoordinator>,
    unread_summary: RwSignal<UnreadSummary>,
    read_receipt_timer: StoredValue<Option<TimeoutControls<()>>>,
    visible_channel: StoredValue<Option<VisibleChannelOwner>>,
    next_visible_owner_id: StoredValue<u64>,
    subscriptions: SubscriptionCoordinator,
    catalog_refresh_epoch: RwSignal<u64>,
    catalog_activity: RwSignal<Option<CatalogActivity>>,
    inbox_request_generation: RwSignal<u64>,
    inbox_retry_timer: StoredValue<Option<TimeoutControls<()>>>,
    inbox_ready: RwSignal<bool>,
    preferences: ChatPreferences,
    session_epoch: RwSignal<u64>,
    identity: Signal<Option<AuthIdentity>>,
    websocket: StoredValue<WebsocketContext>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChatSessionToken {
    session_epoch: u64,
}

impl Chat {
    fn from_identity_signal(
        websocket: WebsocketContext,
        identity: Signal<Option<AuthIdentity>>,
    ) -> Self {
        Self {
            conversations: StoredValue::new(ConversationRegistry::default()),
            unread: StoredValue::new(HashMap::new()),
            read_receipts: StoredValue::new(Default::default()),
            unread_summary: RwSignal::new(UnreadSummary::default()),
            read_receipt_timer: StoredValue::new(None),
            visible_channel: StoredValue::new(None),
            next_visible_owner_id: StoredValue::new(0),
            subscriptions: SubscriptionCoordinator::new(),
            catalog_refresh_epoch: RwSignal::new(0),
            catalog_activity: RwSignal::new(None),
            inbox_request_generation: RwSignal::new(0),
            inbox_retry_timer: StoredValue::new(None),
            inbox_ready: RwSignal::new(false),
            preferences: ChatPreferences::default(),
            session_epoch: RwSignal::new(0),
            identity,
            websocket: StoredValue::new(websocket),
        }
    }

    #[cfg(test)]
    fn new(websocket: WebsocketContext, initial_identity: Option<AuthIdentity>) -> Self {
        Self::from_identity_signal(websocket, Signal::derive(move || initial_identity))
    }

    fn apply_chat_inbox_snapshot(&self, snapshot: ChatInboxSnapshot) {
        let ChatInboxSnapshot {
            blocked_user_ids,
            muted_tournament_ids,
            unread_states,
        } = snapshot;
        self.preferences
            .replace_blocked_user_ids(blocked_user_ids.into_iter().collect());
        let muted_tournament_ids = muted_tournament_ids.into_iter().collect();
        let (newly_muted, newly_unmuted) = self
            .preferences
            .replace_muted_tournament_ids(muted_tournament_ids);
        for tournament_id in newly_muted {
            let key = ConversationKey::tournament(&tournament_id);
            self.clear_unread_state(&key);
            self.set_history_unread_count(&key, 0);
        }
        for tournament_id in newly_unmuted {
            let key = ConversationKey::tournament(&tournament_id);
            if let Some(conversation) = self.conversation_if_exists(&key) {
                conversation.reset_history();
            }
        }
        self.apply_server_unread_states(unread_states);
        self.inbox_ready.set(true);
    }

    fn install_timers(&self) {
        let read_receipt_delay_ms = RwSignal::new(0.0);
        let chat = *self;
        let UseTimeoutFnReturn {
            is_pending: read_receipt_flush_pending,
            start: start_read_receipt_flush,
            stop: stop_read_receipt_flush,
            ..
        } = use_timeout_fn(
            move |_: ()| {
                chat.flush_scheduled_reads();
            },
            read_receipt_delay_ms,
        );
        self.read_receipt_timer.set_value(Some(TimeoutControls::new(
            read_receipt_delay_ms,
            read_receipt_flush_pending,
            start_read_receipt_flush,
            stop_read_receipt_flush,
        )));

        let inbox_retry_delay_ms = RwSignal::new(0.0);
        let chat = *self;
        let UseTimeoutFnReturn {
            is_pending: inbox_retry_pending,
            start: start_inbox_retry,
            stop: stop_inbox_retry,
            ..
        } = use_timeout_fn(
            move |_: ()| {
                chat.retry_chat_inbox_snapshot();
            },
            inbox_retry_delay_ms,
        );
        self.inbox_retry_timer.set_value(Some(TimeoutControls::new(
            inbox_retry_delay_ms,
            inbox_retry_pending,
            start_inbox_retry,
            stop_inbox_retry,
        )));

        let subscription_retry_delay_ms = RwSignal::new(0.0);
        let chat = *self;
        let UseTimeoutFnReturn {
            is_pending: subscription_retry_pending,
            start: start_subscription_retry,
            stop: stop_subscription_retry,
            ..
        } = use_timeout_fn(
            move |_: ()| {
                chat.advance_subscription();
            },
            subscription_retry_delay_ms,
        );
        self.subscriptions.install_retry_timer(TimeoutControls::new(
            subscription_retry_delay_ms,
            subscription_retry_pending,
            start_subscription_retry,
            stop_subscription_retry,
        ));
    }

    fn clear_session_state(&self) {
        self.conversations.update_value(|registry| {
            for conversation in registry.entries.values() {
                conversation.clear_thread();
            }
            registry.entries.clear();
        });
        self.unread.set_value(HashMap::new());
        if let Some(timer) = self.read_receipt_timer.get_value() {
            timer.stop();
        }
        self.read_receipts.set_value(Default::default());
        self.unread_summary.set(UnreadSummary::default());
        self.visible_channel.set_value(None);
        self.stop_inbox_retry();
        bump_generation(self.inbox_request_generation);
        self.inbox_ready.set(false);
        self.preferences.clear();
        bump_generation(self.session_epoch);
    }

    pub(crate) fn session_epoch(&self) -> u64 {
        self.session_epoch.get()
    }

    pub(crate) fn session_epoch_untracked(&self) -> u64 {
        self.session_epoch.get_untracked()
    }

    pub(crate) fn inbox_ready(&self) -> bool {
        self.inbox_ready.get()
    }

    pub(crate) fn identity_untracked(&self) -> Option<AuthIdentity> {
        self.identity.get_untracked()
    }

    pub(crate) fn identity(&self) -> Option<AuthIdentity> {
        self.identity.get()
    }

    pub(crate) fn current_session_token(&self) -> Option<ChatSessionToken> {
        if !matches!(self.identity(), Some(AuthIdentity::User(_))) {
            return None;
        }
        Some(ChatSessionToken {
            session_epoch: self.session_epoch(),
        })
    }

    pub(crate) fn is_current(&self, token: ChatSessionToken) -> bool {
        self.current_session_token() == Some(token)
    }

    fn apply_identity_change(
        &self,
        previous: Option<AuthIdentity>,
        current: Option<AuthIdentity>,
    ) -> bool {
        let changed = previous.is_some() && current.is_some() && previous != current;
        if changed {
            self.clear_session_state();
        }
        changed
    }

    pub(crate) fn current_user_id_untracked(&self) -> Option<Uuid> {
        self.identity_untracked().and_then(AuthIdentity::user_id)
    }

    pub(crate) fn clear_game_thread(&self, game_id: &GameId) {
        let players_key = ConversationKey::game_players(game_id);
        let spectators_key = ConversationKey::game_spectators(game_id);
        self.clear_unread_state(&players_key);
        self.conversations.update_value(|registry| {
            for key in [&players_key, &spectators_key] {
                if let Some(conversation) = registry.entries.get(key) {
                    conversation.clear_thread();
                }
            }
        });
    }

    pub(crate) fn clear_tournament_thread(&self, tournament_id: &TournamentId) {
        let key = ConversationKey::tournament(tournament_id);
        self.remove_unread_state(&key);
        if let Some(conversation) = self.conversation_if_exists(&key) {
            conversation.clear_thread();
        }
    }
}

pub fn provide_chat() {
    let auth = expect_context::<AuthContext>();
    let websocket = expect_context::<WebsocketContext>();
    let chat = Chat::from_identity_signal(websocket.clone(), auth.identity);
    provide_context(chat);
    chat.install_timers();
    let has_opened_connection = StoredValue::new(false);
    Effect::watch(
        move || {
            let identity = auth.identity.get();
            (
                identity,
                websocket.ready_state.get(),
                websocket.wake_resync_epoch.get(),
            )
        },
        move |(identity, ready_state, wake_resync_epoch), previous, _| {
            let identity = *identity;
            let connection_opened = *ready_state == ConnectionReadyState::Open
                && previous.is_none_or(|(_, previous_state, _)| {
                    *previous_state != ConnectionReadyState::Open
                });
            let reconnected = connection_opened && has_opened_connection.get_value();
            if connection_opened {
                has_opened_connection.set_value(true);
            }
            let disconnected = *ready_state != ConnectionReadyState::Open
                && previous.is_some_and(|(_, previous_state, _)| {
                    *previous_state == ConnectionReadyState::Open
                });
            let woke = previous.is_some_and(|(_, _, previous_wake_resync_epoch)| {
                previous_wake_resync_epoch != wake_resync_epoch
            });

            if disconnected {
                chat.mark_pending_outgoing_delivery_unknown();
                chat.stop_inbox_retry();
            }

            let previous_identity = previous.and_then(|(identity, _, _)| *identity);
            let identity_changed = chat.apply_identity_change(previous_identity, identity);

            if reconnected && !identity_changed {
                chat.reset_history();
            }

            let connection_open = *ready_state == ConnectionReadyState::Open;
            chat.update_subscription_prerequisites(connection_open, identity.is_some());

            if identity.is_some() {
                chat.resume_scheduled_reads();
            }

            let identity_became_available = identity.is_some() && previous_identity.is_none();
            if identity.is_some()
                && (identity_became_available || identity_changed || reconnected || woke)
            {
                chat.refresh_chat_inbox_snapshot();
            }
            if identity.is_some() && reconnected {
                chat.request_catalog_refresh();
            }
        },
        true,
    );
}
