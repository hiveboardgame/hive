use super::{bump_generation, Chat};
use crate::{
    functions::chat::get_chat_inbox_snapshot,
    providers::{websocket::ConnectionReadyState, AuthIdentity},
};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{ChatInboxSnapshot, ConversationKey};
use std::time::Duration;

const INBOX_RETRY_DELAY: Duration = Duration::from_secs(10);

impl Chat {
    pub(super) fn refresh_chat_inbox_snapshot(&self) {
        self.stop_inbox_retry();
        match self.identity_untracked() {
            Some(AuthIdentity::Anonymous) => {
                bump_generation(self.inbox_request_generation);
                self.apply_chat_inbox_snapshot(ChatInboxSnapshot::default());
                return;
            }
            Some(AuthIdentity::User(_)) => {}
            None => return,
        }

        let request_generation = bump_generation(self.inbox_request_generation);
        let request_identity = self.identity_untracked();
        let chat = *self;
        spawn_local(async move {
            let snapshot = get_chat_inbox_snapshot().await.ok();
            chat.finish_chat_inbox_snapshot_request(request_generation, request_identity, snapshot);
        });
    }

    pub(super) fn retry_chat_inbox_snapshot(&self) {
        if self.websocket.with_value(|websocket| {
            websocket.ready_state.get_untracked() == ConnectionReadyState::Open
        }) {
            self.refresh_chat_inbox_snapshot();
        }
    }

    pub(super) fn finish_chat_inbox_snapshot_request(
        &self,
        request_generation: u64,
        request_identity: Option<AuthIdentity>,
        snapshot: Option<ChatInboxSnapshot>,
    ) {
        if self.inbox_request_generation.get_untracked() != request_generation
            || self.identity_untracked() != request_identity
        {
            return;
        }
        if let Some(snapshot) = snapshot {
            self.stop_inbox_retry();
            self.apply_chat_inbox_snapshot(snapshot);
        } else {
            self.schedule_inbox_retry();
        }
    }

    fn schedule_inbox_retry(&self) {
        let connection_open = self.websocket.with_value(|websocket| {
            websocket.ready_state.get_untracked() == ConnectionReadyState::Open
        });
        if !connection_open {
            return;
        }
        if let Some(timer) = self.inbox_retry_timer.get_value() {
            timer.schedule(INBOX_RETRY_DELAY, ());
        }
    }

    pub(super) fn stop_inbox_retry(&self) {
        if let Some(timer) = self.inbox_retry_timer.get_value() {
            timer.stop();
        }
    }

    pub(crate) fn request_catalog_refresh(&self) {
        self.catalog_refresh_epoch.update(|epoch| {
            *epoch = epoch.saturating_add(1);
        });
    }

    pub(crate) fn catalog_refresh_epoch(&self) -> u64 {
        self.catalog_refresh_epoch.get()
    }

    pub(crate) fn catalog_activity(&self) -> Option<super::CatalogActivity> {
        self.catalog_activity.get()
    }

    pub(super) fn publish_catalog_activity(&self, key: &ConversationKey, message_id: i64) {
        if !key.tracks_read_receipts() {
            return;
        }
        self.catalog_activity.set(Some(super::CatalogActivity {
            key: key.clone(),
            message_id,
        }));
    }

    pub(crate) fn refresh_inbox_and_catalog(&self) {
        self.refresh_chat_inbox_snapshot();
        self.request_catalog_refresh();
    }
}
