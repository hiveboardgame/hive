use super::{
    history::{HistoryState, InitialHistoryStatus, OlderHistoryStatus},
    outgoing::{OutgoingChat, SendIssue},
    unread::ConversationUnread,
    Chat,
};
use leptos::prelude::*;
use shared_types::{ChatMessage, ConversationKey};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnreadDisplay {
    pub count: i64,
    pub latest_message_id: i64,
}
#[derive(Clone, Debug)]
pub struct ConversationHandle {
    pub(super) key: ConversationKey,
    pub(super) signals: Arc<ConversationSignals>,
    pub(super) unread: ConversationUnread,
    history: ArcStoredValue<HistoryState>,
}

#[derive(Debug, Default)]
pub(super) struct ConversationSignals {
    pub(super) messages: ArcRwSignal<Vec<Arc<ChatMessage>>>,
    pub(super) draft: ArcRwSignal<String>,
    pub(super) send_error: ArcRwSignal<Option<SendIssue>>,
    pub(super) outgoing: ArcRwSignal<Vec<OutgoingChat>>,
}

#[derive(Debug, Default)]
pub(super) struct ConversationRegistry {
    pub(super) entries: HashMap<ConversationKey, ConversationHandle>,
}

impl ConversationHandle {
    pub(super) fn new(key: ConversationKey, unread: ConversationUnread) -> Self {
        Self {
            key,
            signals: Arc::new(ConversationSignals::default()),
            unread,
            history: ArcStoredValue::new(HistoryState::default()),
        }
    }

    pub fn key(&self) -> &ConversationKey {
        &self.key
    }

    pub fn messages(&self) -> ArcRwSignal<Vec<Arc<ChatMessage>>> {
        self.signals.messages.clone()
    }

    pub(crate) fn initial(&self) -> ArcRwSignal<InitialHistoryStatus> {
        self.history_state().initial()
    }

    pub(crate) fn older(&self) -> ArcRwSignal<OlderHistoryStatus> {
        self.history_state().older()
    }

    pub fn draft(&self) -> ArcRwSignal<String> {
        self.signals.draft.clone()
    }

    pub fn send_error(&self) -> ArcRwSignal<Option<SendIssue>> {
        self.signals.send_error.clone()
    }

    pub fn outgoing(&self) -> ArcRwSignal<Vec<OutgoingChat>> {
        self.signals.outgoing.clone()
    }

    pub fn unread(&self) -> ArcRwSignal<UnreadDisplay> {
        self.unread.display.clone()
    }

    pub fn prepend_revision(&self) -> ArcRwSignal<u64> {
        self.history_state().prepend_revision()
    }

    pub(super) fn history_state(&self) -> impl std::ops::DerefMut<Target = HistoryState> {
        self.history.write_value()
    }

    pub(super) fn reset_history(&self) {
        self.history_state().reset();
        self.signals.messages.set(Vec::new());
    }

    pub(super) fn clear_thread(&self) {
        self.reset_history();
        self.signals.draft.set(String::new());
        self.signals.send_error.set(None);
        self.signals.outgoing.set(Vec::new());
        self.unread.clear();
    }
}

impl Chat {
    pub(super) fn unread_entry(&self, key: &ConversationKey) -> ConversationUnread {
        self.unread
            .try_update_value(|registry| registry.entry(key.clone()).or_default().clone())
            .expect("conversation unread storage should not be disposed")
    }

    pub(crate) fn unread(&self, key: &ConversationKey) -> ArcRwSignal<UnreadDisplay> {
        self.unread_entry(key).display
    }

    pub(super) fn conversation_if_exists(
        &self,
        key: &ConversationKey,
    ) -> Option<ConversationHandle> {
        self.conversations
            .with_value(|registry| registry.entries.get(key).cloned())
    }

    pub(crate) fn conversation(&self, key: ConversationKey) -> ConversationHandle {
        let unread = self.unread_entry(&key);
        self.conversations
            .try_update_value(|registry| {
                registry
                    .entries
                    .entry(key.clone())
                    .or_insert_with(|| ConversationHandle::new(key, unread))
                    .clone()
            })
            .expect("conversation storage should not be disposed")
    }
}
