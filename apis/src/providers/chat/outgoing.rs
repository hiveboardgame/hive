use super::{Chat, ConversationHandle};
use crate::{
    common::{ChatSendError, ChatSendRequest, ClientRequest},
    providers::{AlertType, AlertsContext, AuthIdentity},
};
#[cfg(target_arch = "wasm32")]
use leptos::leptos_dom::helpers::set_timeout_with_handle;
use leptos::prelude::*;
use shared_types::{normalize_chat_message, ChatMessageContainer, ConversationKey};
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
const OUTGOING_ACK_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OutgoingState {
    Pending,
    DeliveryUnknown { last_error: Option<SendIssue> },
    Failed { error: SendIssue },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SendIssue {
    LoginRequired,
    ConnectionUnavailable,
    Server(ChatSendError),
}

impl From<ChatSendError> for SendIssue {
    fn from(error: ChatSendError) -> Self {
        Self::Server(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutgoingChat {
    pub(super) client_id: Uuid,
    pub(super) body: String,
    pub(super) turn: Option<usize>,
    pub(super) state: OutgoingState,
    attempt: u64,
}

impl OutgoingChat {
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn state(&self) -> &OutgoingState {
        &self.state
    }
}

impl Chat {
    pub(crate) fn set_draft_message(&self, conversation: &ConversationHandle, message: String) {
        conversation.signals.send_error.set(None);
        conversation.signals.draft.set(message);
    }

    fn queue_pending_outgoing(
        &self,
        conversation: &ConversationHandle,
        client_id: Uuid,
        body: String,
        turn: Option<usize>,
    ) {
        conversation.signals.outgoing.update(|outgoing| {
            outgoing.push(OutgoingChat {
                client_id,
                body,
                turn,
                state: OutgoingState::Pending,
                attempt: 1,
            });
        });
    }

    fn take_outgoing(
        &self,
        conversation: &ConversationHandle,
        client_id: Uuid,
    ) -> Option<OutgoingChat> {
        conversation
            .signals
            .outgoing
            .try_maybe_update(|outgoing| {
                let idx = outgoing
                    .iter()
                    .position(|outgoing| outgoing.client_id == client_id);
                match idx {
                    Some(idx) => (true, Some(outgoing.remove(idx))),
                    None => (false, None),
                }
            })
            .flatten()
    }

    pub(crate) fn handle_failed_chat_send(
        &self,
        key: ConversationKey,
        client_id: Uuid,
        error: SendIssue,
    ) {
        let Some(conversation) = self
            .conversations
            .with_value(|registry| registry.entries.get(&key).cloned())
        else {
            return;
        };
        let restore_message = conversation
            .signals
            .outgoing
            .try_maybe_update(|outgoing| {
                let Some(outgoing) = outgoing
                    .iter_mut()
                    .find(|outgoing| outgoing.client_id == client_id)
                else {
                    return (false, None);
                };
                let (state, restore_message) = match &outgoing.state {
                    OutgoingState::Pending
                        if outgoing.attempt == 1
                            && error == SendIssue::Server(ChatSendError::Unavailable) =>
                    {
                        (
                            OutgoingState::DeliveryUnknown {
                                last_error: Some(error.clone()),
                            },
                            None,
                        )
                    }
                    OutgoingState::Pending if outgoing.attempt == 1 => (
                        OutgoingState::Failed {
                            error: error.clone(),
                        },
                        Some(outgoing.body.clone()),
                    ),
                    OutgoingState::Pending | OutgoingState::DeliveryUnknown { .. } => {
                        let state = if error == SendIssue::Server(ChatSendError::ClientIdConflict) {
                            OutgoingState::Failed {
                                error: error.clone(),
                            }
                        } else {
                            OutgoingState::DeliveryUnknown {
                                last_error: Some(error.clone()),
                            }
                        };
                        (state, None)
                    }
                    OutgoingState::Failed { .. } => return (false, None),
                };
                outgoing.state = state;
                (true, restore_message)
            })
            .flatten();
        if let Some(message) = restore_message {
            if conversation.signals.draft.get_untracked().is_empty() {
                conversation.signals.draft.set(message);
            }
        }
    }

    pub(crate) fn dismiss_outgoing(
        &self,
        conversation: &ConversationHandle,
        client_id: Uuid,
    ) -> bool {
        conversation
            .signals
            .outgoing
            .try_maybe_update(|outgoing| {
                let idx = outgoing.iter().position(|entry| {
                    entry.client_id == client_id
                        && matches!(
                            &entry.state,
                            OutgoingState::DeliveryUnknown { .. } | OutgoingState::Failed { .. }
                        )
                });
                if let Some(idx) = idx {
                    outgoing.remove(idx);
                    (true, true)
                } else {
                    (false, false)
                }
            })
            .unwrap_or(false)
    }

    pub(crate) fn send(&self, conversation: &ConversationHandle, turn: Option<usize>) -> bool {
        let identity = self.identity_untracked();
        let Some(AuthIdentity::User(_)) = identity else {
            if identity == Some(AuthIdentity::Anonymous) {
                conversation
                    .signals
                    .send_error
                    .set(Some(SendIssue::LoginRequired));
            }
            return false;
        };
        let body = normalize_chat_message(&conversation.signals.draft.get_untracked());
        if body.trim().is_empty() {
            return false;
        }
        let turn = match conversation.key() {
            ConversationKey::Game { .. } => turn,
            _ => None,
        };
        let client_message_id = Uuid::new_v4();
        conversation.signals.send_error.set(None);
        let request = ChatSendRequest {
            key: conversation.key.clone(),
            client_id: client_message_id,
            body: body.clone(),
            turn,
        };
        self.queue_pending_outgoing(conversation, client_message_id, body, turn);
        if !self
            .websocket
            .with_value(|websocket| websocket.send(&ClientRequest::Chat(request)))
        {
            self.take_outgoing(conversation, client_message_id);
            conversation
                .signals
                .send_error
                .set(Some(SendIssue::ConnectionUnavailable));
            return false;
        }
        conversation.signals.draft.set(String::new());
        self.schedule_outgoing_timeout(conversation.clone(), client_message_id, 1);
        true
    }

    pub(crate) fn retry_outgoing(
        &self,
        conversation: &ConversationHandle,
        client_id: Uuid,
    ) -> bool {
        if !matches!(self.identity_untracked(), Some(AuthIdentity::User(_))) {
            conversation.signals.outgoing.try_maybe_update(|entries| {
                let Some(entry) = entries.iter_mut().find(|entry| {
                    entry.client_id == client_id
                        && matches!(entry.state, OutgoingState::DeliveryUnknown { .. })
                }) else {
                    return (false, ());
                };
                entry.state = OutgoingState::DeliveryUnknown {
                    last_error: Some(SendIssue::LoginRequired),
                };
                (true, ())
            });
            return false;
        }
        let retry = conversation
            .signals
            .outgoing
            .try_maybe_update(|entries| {
                let Some(entry) = entries.iter_mut().find(|entry| {
                    entry.client_id == client_id
                        && matches!(entry.state, OutgoingState::DeliveryUnknown { .. })
                }) else {
                    return (false, None);
                };
                entry.attempt = entry.attempt.saturating_add(1);
                let request = ChatSendRequest {
                    key: conversation.key.clone(),
                    client_id: entry.client_id,
                    body: entry.body.clone(),
                    turn: entry.turn,
                };
                let sent = self
                    .websocket
                    .with_value(|websocket| websocket.send(&ClientRequest::Chat(request)));
                entry.state = if sent {
                    OutgoingState::Pending
                } else {
                    OutgoingState::DeliveryUnknown {
                        last_error: Some(SendIssue::ConnectionUnavailable),
                    }
                };
                (true, Some((sent, entry.attempt)))
            })
            .flatten();
        let Some((sent, attempt)) = retry else {
            return false;
        };
        if !sent {
            return false;
        }
        self.schedule_outgoing_timeout(conversation.clone(), client_id, attempt);
        true
    }

    fn schedule_outgoing_timeout(
        &self,
        conversation: ConversationHandle,
        client_id: Uuid,
        attempt: u64,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            let chat = *self;
            let _ = set_timeout_with_handle(
                move || {
                    chat.mark_outgoing_attempt_timed_out(&conversation, client_id, attempt);
                },
                OUTGOING_ACK_TIMEOUT,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (conversation, client_id, attempt);
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn mark_outgoing_attempt_timed_out(
        &self,
        conversation: &ConversationHandle,
        client_id: Uuid,
        attempt: u64,
    ) -> bool {
        conversation
            .signals
            .outgoing
            .try_maybe_update(|entries| {
                let Some(entry) = entries
                    .iter_mut()
                    .find(|entry| entry.client_id == client_id)
                else {
                    return (false, false);
                };
                if entry.attempt != attempt || !matches!(&entry.state, OutgoingState::Pending) {
                    return (false, false);
                }
                entry.state = OutgoingState::DeliveryUnknown { last_error: None };
                (true, true)
            })
            .unwrap_or(false)
    }

    pub(super) fn mark_pending_outgoing_delivery_unknown(&self) {
        self.conversations.with_value(|registry| {
            for conversation in registry.entries.values() {
                conversation.signals.outgoing.try_maybe_update(|outgoing| {
                    let mut changed = false;
                    for outgoing in outgoing {
                        if matches!(&outgoing.state, OutgoingState::Pending) {
                            outgoing.state = OutgoingState::DeliveryUnknown { last_error: None };
                            changed = true;
                        }
                    }
                    (changed, ())
                });
            }
        });
    }

    pub(crate) fn recv(&self, container: ChatMessageContainer) {
        if self.identity_untracked().is_none() {
            return;
        }
        let key = container.key.clone();
        let conversation = self.conversation_if_exists(&key);
        let message_user_id = container.message.user_id;
        let from_self = self.current_user_id_untracked() == Some(message_user_id);
        let persisted_message_id = container.message.id;
        let is_global = matches!(&container.key, ConversationKey::Global);
        let alert_message = is_global.then(|| container.message.message.clone());

        if from_self {
            if let Some(conversation) = &conversation {
                self.take_outgoing(conversation, container.client_id);
            }
            if key.tracks_read_receipts() {
                self.record_authoritative_read(&key, persisted_message_id);
            }
        }

        if let Some(conversation) = &conversation {
            let inserted = self.insert_message(conversation, Arc::new(container.message));
            if !inserted {
                return;
            }
        }
        self.publish_catalog_activity(&key, persisted_message_id);

        if is_global {
            let alerts = expect_context::<AlertsContext>();
            alerts.last_alert.update(|alert| {
                *alert = Some(AlertType::Warn(alert_message.unwrap_or_default()));
            });
        }

        let tracks_unread = self.tracks_unread(&key);
        if !from_self && tracks_unread && persisted_message_id > self.read_floor_untracked(&key) {
            self.add_local_unread(&key, persisted_message_id);
            self.increment_history_unread_count(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthIdentity, Chat, OutgoingState, SendIssue};
    use crate::{
        common::{ChatSendError, ChatSendRequest, ClientRequest, ServerResult},
        providers::websocket::{ConnectionReadyState, WebsocketContext},
    };
    use chrono::Utc;
    use leptos::prelude::*;
    use shared_types::{ChatMessage, ChatMessageContainer, ConversationKey, GameId, GameThread};
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    fn chat_with_send(
        user_id: Uuid,
        send: impl Fn(&ClientRequest) -> bool + Send + Sync + 'static,
    ) -> Chat {
        let websocket = WebsocketContext::new(
            Signal::derive(|| None::<ServerResult>),
            Arc::new(send),
            Signal::derive(|| ConnectionReadyState::Open),
            Arc::new(|| {}),
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        Chat::new(websocket, Some(AuthIdentity::User(user_id)))
    }

    fn send(chat: &Chat, message: &str, key: ConversationKey, turn: Option<usize>) -> bool {
        let conversation = chat.conversation(key);
        chat.set_draft_message(&conversation, message.to_string());
        chat.send(&conversation, turn)
    }

    fn captured_chat_requests(requests: &Arc<Mutex<Vec<ClientRequest>>>) -> Vec<ChatSendRequest> {
        requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter_map(|request| match request {
                ClientRequest::Chat(request) => Some(request.clone()),
                _ => None,
            })
            .collect()
    }

    fn authoritative_echo(
        key: ConversationKey,
        user_id: Uuid,
        client_id: Uuid,
        message_id: i64,
        body: &str,
        turn: Option<usize>,
    ) -> ChatMessageContainer {
        let message = ChatMessage {
            id: message_id,
            user_id,
            username: "current".to_string(),
            timestamp: Utc::now(),
            message: body.to_string(),
            turn,
        };
        ChatMessageContainer::new(key, message, client_id)
    }

    #[test]
    fn unavailable_and_disconnect_mark_requests_delivery_unknown() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_send = Arc::clone(&requests);
        let chat = chat_with_send(user_id, move |request| {
            requests_for_send
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(request.clone());
            true
        });
        let pending_key = ConversationKey::direct(Uuid::new_v4());
        let retry_key = ConversationKey::direct(Uuid::new_v4());
        let failed_key = ConversationKey::direct(Uuid::new_v4());
        assert!(send(&chat, "failed", failed_key.clone(), None));
        let failed_client_id = chat
            .conversation(failed_key.clone())
            .outgoing()
            .get_untracked()[0]
            .client_id();
        chat.handle_failed_chat_send(
            failed_key.clone(),
            failed_client_id,
            SendIssue::Server(ChatSendError::Unavailable),
        );
        assert!(send(&chat, "retry", retry_key.clone(), None));
        chat.mark_pending_outgoing_delivery_unknown();
        let retry_client_id = chat
            .conversation(retry_key.clone())
            .outgoing()
            .get_untracked()[0]
            .client_id();
        let retry_conversation = chat.conversation(retry_key.clone());
        assert!(chat.retry_outgoing(&retry_conversation, retry_client_id));
        assert!(send(&chat, "pending", pending_key.clone(), None));

        chat.mark_pending_outgoing_delivery_unknown();

        assert_eq!(
            chat.conversation(pending_key).outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown { last_error: None },
        );
        assert_eq!(
            chat.conversation(retry_key).outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown { last_error: None },
        );
        assert_eq!(
            chat.conversation(failed_key).outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown {
                last_error: Some(SendIssue::Server(ChatSendError::Unavailable)),
            },
        );
    }

    #[test]
    fn retry_reuses_the_exact_client_id_and_payload_then_reconciles_one_echo() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_send = Arc::clone(&requests);
        let websocket = WebsocketContext::new(
            Signal::derive(|| None::<ServerResult>),
            Arc::new(move |request| {
                requests_for_send
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(request.clone());
                true
            }),
            Signal::derive(|| ConnectionReadyState::Open),
            Arc::new(|| {}),
            Arc::new(|| {}),
            Arc::new(|| {}),
        );
        let chat = Chat::new(websocket, Some(AuthIdentity::User(user_id)));
        let key = ConversationKey::Game {
            game_id: GameId("delivery-retry".to_string()),
            thread: GameThread::Spectators,
        };
        assert!(send(&chat, "same payload", key.clone(), Some(17)));
        chat.mark_pending_outgoing_delivery_unknown();
        let initial = captured_chat_requests(&requests)[0].clone();
        let client_id = initial.client_id;
        let conversation = chat.conversation(key.clone());
        assert!(chat.retry_outgoing(&conversation, client_id));
        let sent = captured_chat_requests(&requests);
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[1], initial);
        assert_eq!(
            chat.conversation(key.clone()).outgoing().get_untracked()[0].state(),
            &OutgoingState::Pending,
        );

        let echo = authoritative_echo(
            key.clone(),
            user_id,
            client_id,
            41,
            "same payload",
            Some(17),
        );
        chat.recv(echo.clone());
        chat.recv(echo);

        let conversation = chat.conversation(key);
        assert!(conversation.outgoing().get_untracked().is_empty());
        assert_eq!(conversation.messages().get_untracked().len(), 1);
        assert_eq!(conversation.messages().get_untracked()[0].id, 41);
    }

    #[test]
    fn current_attempt_times_out_stale_attempt_does_not_and_late_echo_reconciles() {
        let owner = Owner::new();
        owner.set();
        let user_id = Uuid::new_v4();
        let chat = chat_with_send(user_id, |_| true);
        let key = ConversationKey::direct(Uuid::new_v4());
        assert!(send(&chat, "uncertain delivery", key.clone(), None));
        let conversation = chat.conversation(key.clone());
        let client_id = conversation.outgoing().get_untracked()[0].client_id();

        assert!(chat.mark_outgoing_attempt_timed_out(&conversation, client_id, 1));
        assert_eq!(
            conversation.outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown { last_error: None },
        );
        assert!(chat.retry_outgoing(&conversation, client_id));
        assert!(!chat.mark_outgoing_attempt_timed_out(&conversation, client_id, 1));
        assert_eq!(
            conversation.outgoing().get_untracked()[0].state(),
            &OutgoingState::Pending,
        );
        assert!(chat.mark_outgoing_attempt_timed_out(&conversation, client_id, 2));

        chat.recv(authoritative_echo(
            key,
            user_id,
            client_id,
            77,
            "uncertain delivery",
            None,
        ));
        assert!(conversation.outgoing().get_untracked().is_empty());
    }

    #[test]
    fn enqueue_and_retry_failures_preserve_the_correct_recovery_state() {
        let owner = Owner::new();
        owner.set();
        let outcomes = Arc::new(Mutex::new(VecDeque::from([false, true, false, true, true])));
        let outcomes_for_send = Arc::clone(&outcomes);
        let chat = chat_with_send(Uuid::new_v4(), move |_| {
            outcomes_for_send
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pop_front()
                .expect("configured send outcome")
        });

        let initial_failure = chat.conversation(ConversationKey::direct(Uuid::new_v4()));
        chat.set_draft_message(&initial_failure, "initial enqueue".to_string());
        assert!(!chat.send(&initial_failure, None));
        assert_eq!(initial_failure.draft().get_untracked(), "initial enqueue");
        assert!(initial_failure.outgoing().get_untracked().is_empty());
        assert_eq!(
            initial_failure.send_error().get_untracked(),
            Some(SendIssue::ConnectionUnavailable),
        );

        let retry_failure = chat.conversation(ConversationKey::direct(Uuid::new_v4()));
        chat.set_draft_message(&retry_failure, "retry enqueue".to_string());
        assert!(chat.send(&retry_failure, None));
        assert_eq!(retry_failure.draft().get_untracked(), "");
        chat.mark_pending_outgoing_delivery_unknown();
        let retry_failure_id = retry_failure.outgoing().get_untracked()[0].client_id();
        assert!(!chat.retry_outgoing(&retry_failure, retry_failure_id));
        assert_eq!(
            retry_failure.outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown {
                last_error: Some(SendIssue::ConnectionUnavailable),
            },
        );

        let server_failure_key = ConversationKey::direct(Uuid::new_v4());
        let server_failure = chat.conversation(server_failure_key.clone());
        chat.set_draft_message(&server_failure, "server retry".to_string());
        assert!(chat.send(&server_failure, None));
        assert_eq!(server_failure.draft().get_untracked(), "");
        chat.mark_pending_outgoing_delivery_unknown();
        let server_failure_id = server_failure.outgoing().get_untracked()[0].client_id();
        assert!(chat.retry_outgoing(&server_failure, server_failure_id));
        let server_error = SendIssue::Server(ChatSendError::RateLimited);
        chat.handle_failed_chat_send(server_failure_key, server_failure_id, server_error.clone());
        assert_eq!(
            server_failure.outgoing().get_untracked()[0].state(),
            &OutgoingState::DeliveryUnknown {
                last_error: Some(server_error),
            },
        );
        assert_eq!(server_failure.draft().get_untracked(), "");
        assert!(outcomes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty());
    }

    #[test]
    fn explicit_failure_retains_failed_and_protects_newer_draft() {
        let owner = Owner::new();
        owner.set();
        let chat = chat_with_send(Uuid::new_v4(), |_| true);
        let newer_key = ConversationKey::direct(Uuid::new_v4());
        let restore_key = ConversationKey::direct(Uuid::new_v4());
        assert!(send(&chat, "old draft", newer_key.clone(), None));
        assert!(send(&chat, "restore me", restore_key.clone(), None));
        let newer = chat.conversation(newer_key.clone());
        let restore = chat.conversation(restore_key.clone());
        let newer_client_id = newer.outgoing().get_untracked()[0].client_id();
        let restore_client_id = restore.outgoing().get_untracked()[0].client_id();
        chat.set_draft_message(&newer, "new typing".to_string());

        chat.handle_failed_chat_send(
            newer_key,
            newer_client_id,
            SendIssue::Server(ChatSendError::RateLimited),
        );
        chat.handle_failed_chat_send(
            restore_key.clone(),
            restore_client_id,
            SendIssue::Server(ChatSendError::DirectRestricted),
        );

        assert_eq!(newer.draft().get_untracked(), "new typing");
        assert_eq!(restore.draft().get_untracked(), "restore me");
        assert_eq!(
            newer.outgoing().get_untracked()[0].state(),
            &OutgoingState::Failed {
                error: SendIssue::Server(ChatSendError::RateLimited),
            },
        );
        assert_eq!(
            restore.outgoing().get_untracked()[0].state(),
            &OutgoingState::Failed {
                error: SendIssue::Server(ChatSendError::DirectRestricted),
            },
        );
        assert!(chat.dismiss_outgoing(&restore, restore_client_id));
        assert!(restore.outgoing().get_untracked().is_empty());
        assert_eq!(restore.send_error().get_untracked(), None);
    }
}
