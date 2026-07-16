use super::{Chat, ConversationHandle};
use crate::{functions::chat::get_chat_history, providers::AuthIdentity};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{ChatHistoryPage, ChatHistoryResponse, ChatMessage, ConversationKey};
use std::{collections::HashSet, sync::Arc};

const GLOBAL_HISTORY_LIMIT: i64 = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum InitialHistoryStatus {
    #[default]
    NotLoaded,
    Loading(InitialRequest),
    Ready {
        unread_anchor: Option<i64>,
        next_before_message_id: Option<i64>,
    },
    AccessDenied,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum OlderHistoryStatus {
    #[default]
    Idle,
    Loading(OlderRequest),
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InitialRequest {
    pub(super) request_id: u64,
    pub(super) read_floor: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OlderRequest {
    pub(super) request_id: u64,
    pub(super) before_message_id: i64,
}

#[derive(Debug, Default)]
pub(super) struct HistoryState {
    initial: ArcRwSignal<InitialHistoryStatus>,
    older: ArcRwSignal<OlderHistoryStatus>,
    prepend_revision: ArcRwSignal<u64>,
    pub(super) next_request_id: u64,
}

impl HistoryState {
    pub(super) fn initial(&self) -> ArcRwSignal<InitialHistoryStatus> {
        self.initial.clone()
    }

    pub(super) fn older(&self) -> ArcRwSignal<OlderHistoryStatus> {
        self.older.clone()
    }

    pub(super) fn prepend_revision(&self) -> ArcRwSignal<u64> {
        self.prepend_revision.clone()
    }

    pub(super) fn begin_initial(&mut self, read_floor: i64) -> Option<InitialRequest> {
        if !matches!(
            self.initial.get_untracked(),
            InitialHistoryStatus::NotLoaded
                | InitialHistoryStatus::AccessDenied
                | InitialHistoryStatus::Failed
        ) {
            return None;
        }
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = InitialRequest {
            request_id: self.next_request_id,
            read_floor,
        };
        self.initial
            .set(InitialHistoryStatus::Loading(request.clone()));
        self.older.set(OlderHistoryStatus::Idle);
        Some(request)
    }

    pub(super) fn begin_older(&mut self) -> Option<OlderRequest> {
        let before_message_id = match self.initial.get_untracked() {
            InitialHistoryStatus::Ready {
                next_before_message_id: Some(before_message_id),
                ..
            } => before_message_id,
            _ => return None,
        };
        if matches!(self.older.get_untracked(), OlderHistoryStatus::Loading(_)) {
            return None;
        }
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = OlderRequest {
            request_id: self.next_request_id,
            before_message_id,
        };
        self.older.set(OlderHistoryStatus::Loading(request.clone()));
        Some(request)
    }

    fn initial_is_loading(&self, request: &InitialRequest) -> bool {
        self.initial.with_untracked(
            |status| matches!(status, InitialHistoryStatus::Loading(current) if current == request),
        )
    }

    fn older_is_loading(&self, request: &OlderRequest) -> bool {
        self.older.with_untracked(
            |status| matches!(status, OlderHistoryStatus::Loading(current) if current == request),
        )
    }

    pub(super) fn finish_initial_page(
        &mut self,
        request: &InitialRequest,
        next_before_message_id: Option<i64>,
        unread_anchor: Option<i64>,
    ) -> bool {
        if !self.initial_is_loading(request) {
            return false;
        }
        self.initial.set(InitialHistoryStatus::Ready {
            unread_anchor,
            next_before_message_id,
        });
        self.older.set(OlderHistoryStatus::Idle);
        true
    }

    pub(super) fn finish_older_page(
        &mut self,
        request: &OlderRequest,
        next_before_message_id: Option<i64>,
    ) -> bool {
        if !self.older_is_loading(request) {
            return false;
        }
        let unread_anchor = match self.initial.get_untracked() {
            InitialHistoryStatus::Ready { unread_anchor, .. } => unread_anchor,
            _ => None,
        };
        self.initial.set(InitialHistoryStatus::Ready {
            unread_anchor,
            next_before_message_id,
        });
        self.older.set(OlderHistoryStatus::Idle);
        true
    }

    pub(super) fn record_prepend(&mut self) {
        self.prepend_revision
            .update(|revision| *revision = revision.saturating_add(1));
    }

    pub(super) fn fail_initial(&mut self, request: &InitialRequest, error: String) -> bool {
        if !self.initial_is_loading(request) {
            return false;
        }
        log::error!("failed to load initial chat history: {error}");
        self.initial.set(InitialHistoryStatus::Failed);
        self.older.set(OlderHistoryStatus::Idle);
        true
    }

    pub(super) fn fail_older(&mut self, request: &OlderRequest, error: String) -> bool {
        if !self.older_is_loading(request) {
            return false;
        }
        log::error!("failed to load older chat history: {error}");
        self.older.set(OlderHistoryStatus::Failed);
        true
    }

    pub(super) fn deny_initial(&mut self, request: &InitialRequest) -> bool {
        if !self.initial_is_loading(request) {
            return false;
        }
        self.deny_access();
        true
    }

    pub(super) fn deny_older(&mut self, request: &OlderRequest) -> bool {
        if !self.older_is_loading(request) {
            return false;
        }
        self.deny_access();
        true
    }

    fn deny_access(&mut self) {
        self.initial.set(InitialHistoryStatus::AccessDenied);
        self.older.set(OlderHistoryStatus::Idle);
    }

    pub(super) fn prepare_initial_retry(&mut self) -> bool {
        if !matches!(self.initial.get_untracked(), InitialHistoryStatus::Failed) {
            return false;
        }
        self.initial.set(InitialHistoryStatus::NotLoaded);
        self.older.set(OlderHistoryStatus::Idle);
        true
    }

    pub(super) fn reset(&mut self) {
        self.initial.set(InitialHistoryStatus::NotLoaded);
        self.older.set(OlderHistoryStatus::Idle);
    }

    pub(super) fn set_unread_anchor(&mut self, unread_anchor: Option<i64>) {
        if let InitialHistoryStatus::Ready {
            next_before_message_id,
            ..
        } = self.initial.get_untracked()
        {
            self.initial.set(InitialHistoryStatus::Ready {
                unread_anchor,
                next_before_message_id,
            });
        }
    }

    pub(super) fn increment_unread_anchor(&mut self) {
        if let InitialHistoryStatus::Ready {
            unread_anchor,
            next_before_message_id,
        } = self.initial.get_untracked()
        {
            self.initial.set(InitialHistoryStatus::Ready {
                unread_anchor: Some(unread_anchor.unwrap_or(0).saturating_add(1)),
                next_before_message_id,
            });
        }
    }
}

impl Chat {
    pub(super) fn reset_history(&self) {
        self.conversations.with_value(|registry| {
            for conversation in registry.entries.values() {
                conversation.reset_history();
            }
        });
    }

    pub(crate) fn ensure_initial_history(&self, conversation: ConversationHandle) {
        if self.identity_untracked().is_none() {
            return;
        }
        let Some(request) = self.begin_initial_history_request(&conversation) else {
            return;
        };

        let chat = *self;
        spawn_local(async move {
            let result = get_chat_history(conversation.key.clone(), None)
                .await
                .map_err(|error| error.to_string());
            chat.apply_initial_history_result(&conversation, &request, result);
        });
    }

    pub(super) fn begin_initial_history_request(
        &self,
        conversation: &ConversationHandle,
    ) -> Option<InitialRequest> {
        let read_floor = self.read_floor_untracked(conversation.key());
        conversation.history_state().begin_initial(read_floor)
    }

    pub(crate) fn retry_initial_history(&self, conversation: ConversationHandle) {
        if conversation.history_state().prepare_initial_retry() {
            self.ensure_initial_history(conversation);
        }
    }

    pub(crate) fn load_older_history(&self, conversation: ConversationHandle) -> bool {
        if self.identity_untracked().is_none() {
            return false;
        }
        let Some(request) = self.begin_older_history_request(&conversation) else {
            return false;
        };

        let chat = *self;
        spawn_local(async move {
            let result =
                get_chat_history(conversation.key.clone(), Some(request.before_message_id))
                    .await
                    .map_err(|error| error.to_string());
            chat.apply_older_history_result(&conversation, &request, result);
        });
        true
    }

    pub(super) fn begin_older_history_request(
        &self,
        conversation: &ConversationHandle,
    ) -> Option<OlderRequest> {
        conversation.history_state().begin_older()
    }

    pub(crate) fn latest_cached_message_id_untracked(
        &self,
        conversation: &ConversationHandle,
    ) -> i64 {
        conversation
            .signals
            .messages
            .with_untracked(|messages| messages.last().map(|message| message.id))
            .unwrap_or(0)
    }

    pub(super) fn apply_initial_history_result(
        &self,
        conversation: &ConversationHandle,
        request: &InitialRequest,
        result: Result<ChatHistoryResponse, String>,
    ) -> bool {
        match result {
            Ok(ChatHistoryResponse::Page(page)) => {
                self.apply_initial_history_page(conversation, request, page)
            }
            Ok(ChatHistoryResponse::AccessDenied) => {
                conversation.history_state().deny_initial(request)
            }
            Err(error) => conversation.history_state().fail_initial(request, error),
        }
    }

    pub(super) fn apply_older_history_result(
        &self,
        conversation: &ConversationHandle,
        request: &OlderRequest,
        result: Result<ChatHistoryResponse, String>,
    ) -> bool {
        match result {
            Ok(ChatHistoryResponse::Page(page)) => {
                self.apply_older_history_page(conversation, request, page)
            }
            Ok(ChatHistoryResponse::AccessDenied) => {
                conversation.history_state().deny_older(request)
            }
            Err(error) => conversation.history_state().fail_older(request, error),
        }
    }

    fn apply_initial_history_page(
        &self,
        conversation: &ConversationHandle,
        request: &InitialRequest,
        page: ChatHistoryPage,
    ) -> bool {
        let next_before_message_id = if page.messages.is_empty() {
            None
        } else {
            page.next_before_message_id
        };
        if !conversation.history_state().finish_initial_page(
            request,
            next_before_message_id,
            page.initial_unread_count,
        ) {
            return false;
        }
        self.merge_messages(conversation, page.messages);

        let read_floor = self.read_floor_untracked(conversation.key());
        if page.initial_unread_count.is_some() && read_floor > request.read_floor {
            let current_user_id = self.identity_untracked().and_then(AuthIdentity::user_id);
            let unread_anchor = conversation.signals.messages.with_untracked(|messages| {
                messages
                    .iter()
                    .filter(|message| {
                        message.id > read_floor && Some(message.user_id) != current_user_id
                    })
                    .count() as i64
            });
            conversation
                .history_state()
                .set_unread_anchor(Some(unread_anchor));
        }
        true
    }

    fn apply_older_history_page(
        &self,
        conversation: &ConversationHandle,
        request: &OlderRequest,
        page: ChatHistoryPage,
    ) -> bool {
        let next_before_message_id = if page.messages.is_empty() {
            None
        } else {
            page.next_before_message_id
        };
        if !conversation
            .history_state()
            .finish_older_page(request, next_before_message_id)
        {
            return false;
        }
        self.merge_messages(conversation, page.messages);
        conversation.history_state().record_prepend();
        true
    }

    pub(super) fn merge_messages(
        &self,
        conversation: &ConversationHandle,
        incoming: Vec<ChatMessage>,
    ) -> bool {
        conversation
            .signals
            .messages
            .try_maybe_update(|messages| {
                let mut persisted_ids = messages
                    .iter()
                    .map(|message| message.id)
                    .collect::<HashSet<_>>();
                let mut inserted_any = false;
                for message in incoming {
                    let message_id = message.id;
                    if persisted_ids.insert(message_id) {
                        messages.push(Arc::new(message));
                        inserted_any = true;
                    }
                }
                if inserted_any {
                    messages.sort_by_key(|message| message.id);
                    if matches!(conversation.key, ConversationKey::Global)
                        && messages.len() > GLOBAL_HISTORY_LIMIT as usize
                    {
                        let drop_count = messages.len() - GLOBAL_HISTORY_LIMIT as usize;
                        messages.drain(..drop_count);
                    }
                }
                (inserted_any, inserted_any)
            })
            .unwrap_or(false)
    }

    pub(super) fn insert_message(
        &self,
        conversation: &ConversationHandle,
        message: Arc<ChatMessage>,
    ) -> bool {
        let message_id = message.id;
        conversation
            .signals
            .messages
            .try_maybe_update(|messages| {
                if messages.iter().any(|existing| existing.id == message_id) {
                    return (false, false);
                }

                let insert_at = messages.partition_point(|existing| existing.id <= message_id);
                messages.insert(insert_at, message);
                if matches!(conversation.key, ConversationKey::Global)
                    && messages.len() > GLOBAL_HISTORY_LIMIT as usize
                {
                    messages.remove(0);
                }
                (true, true)
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryState, InitialHistoryStatus, OlderHistoryStatus};
    use leptos::prelude::*;

    #[test]
    fn initial_completion_rejects_wrong_request_reset_and_replay() {
        let mut history = HistoryState::default();
        let request = history.begin_initial(4).unwrap();
        let mut wrong_token = request.clone();
        wrong_token.request_id += 1;

        assert!(!history.finish_initial_page(&wrong_token, None, Some(2)));
        history.reset();
        assert!(!history.finish_initial_page(&request, None, Some(2)));

        let current = history.begin_initial(4).unwrap();
        assert!(history.finish_initial_page(&current, None, Some(2)));
        assert!(!history.finish_initial_page(&current, None, Some(9)));
        assert_eq!(
            history.initial().get_untracked(),
            InitialHistoryStatus::Ready {
                unread_anchor: Some(2),
                next_before_message_id: None,
            },
        );
    }

    #[test]
    fn initial_failure_can_retry_with_a_new_request_id() {
        let mut history = HistoryState::default();
        let failed = history.begin_initial(0).unwrap();

        assert!(history.fail_initial(&failed, "temporary".to_string()));
        assert_eq!(
            history.initial().get_untracked(),
            InitialHistoryStatus::Failed,
        );
        assert!(history.prepare_initial_retry());
        assert_eq!(
            history.initial().get_untracked(),
            InitialHistoryStatus::NotLoaded
        );

        let retry = history.begin_initial(0).unwrap();
        assert!(retry.request_id > failed.request_id);
    }

    #[test]
    fn older_failure_retries_the_same_cursor_with_a_new_token() {
        let mut history = HistoryState::default();
        let initial = history.begin_initial(0).unwrap();
        assert!(history.finish_initial_page(&initial, Some(51), Some(0)));
        let first = history.begin_older().unwrap();
        let mut wrong_request = first.clone();
        wrong_request.request_id += 1;

        assert!(history.begin_older().is_none());
        assert!(!history.fail_older(&wrong_request, "wrong request".to_string()));
        assert!(history.fail_older(&first, "temporary".to_string()));
        assert_eq!(history.older().get_untracked(), OlderHistoryStatus::Failed,);

        let retry = history.begin_older().unwrap();
        assert_eq!(retry.before_message_id, 51);
        assert!(retry.request_id > first.request_id);
        assert!(!history.fail_older(&first, "stale".to_string()));
    }

    #[test]
    fn stale_older_request_cannot_complete_after_reset() {
        let mut history = HistoryState::default();
        let initial = history.begin_initial(0).unwrap();
        assert!(history.finish_initial_page(&initial, Some(51), Some(0)));
        let older = history.begin_older().unwrap();

        history.reset();

        assert!(!history.finish_older_page(&older, None));
        assert_eq!(
            history.initial().get_untracked(),
            InitialHistoryStatus::NotLoaded
        );
        assert_eq!(history.prepend_revision().get_untracked(), 0);
    }
}
