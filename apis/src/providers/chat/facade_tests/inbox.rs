use super::fixtures::*;
use crate::{
    common::ServerResult,
    providers::websocket::{ConnectionReadyState, WebsocketContext},
};
use shared_types::ChatInboxSnapshot;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn chat_with_connection(user_id: Uuid, ready_state: RwSignal<ConnectionReadyState>) -> Chat {
    let websocket = WebsocketContext::new(
        Signal::derive(|| None::<ServerResult>),
        Arc::new(|_| true),
        ready_state.into(),
        Arc::new(|| {}),
        Arc::new(|| {}),
        Arc::new(|| {}),
    );
    Chat::new(websocket, Some(AuthIdentity::User(user_id)))
}

fn install_inbox_retry_timer(chat: Chat) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let starts = Arc::new(AtomicUsize::new(0));
    let stops = Arc::new(AtomicUsize::new(0));
    let starts_for_timer = Arc::clone(&starts);
    let stops_for_timer = Arc::clone(&stops);
    chat.inbox_retry_timer
        .set_value(Some(super::super::TimeoutControls::new(
            RwSignal::new(0.0),
            Signal::derive(|| false),
            move |_: ()| {
                starts_for_timer.fetch_add(1, Ordering::Relaxed);
            },
            move || {
                stops_for_timer.fetch_add(1, Ordering::Relaxed);
            },
        )));
    (starts, stops)
}

#[test]
fn current_inbox_failure_retries_only_while_connected() {
    let owner = Owner::new();
    owner.set();
    let ready_state = RwSignal::new(ConnectionReadyState::Open);
    let chat = chat_with_connection(Uuid::new_v4(), ready_state);
    let (starts, _) = install_inbox_retry_timer(chat);
    let request_generation = chat.inbox_request_generation.get_untracked();
    let request_identity = chat.identity_untracked();

    chat.finish_chat_inbox_snapshot_request(request_generation, request_identity, None);
    assert_eq!(starts.load(Ordering::Relaxed), 1);

    ready_state.set(ConnectionReadyState::Closed);
    chat.finish_chat_inbox_snapshot_request(request_generation, request_identity, None);
    assert_eq!(starts.load(Ordering::Relaxed), 1);
}

#[test]
fn stale_inbox_failure_does_not_schedule_retry() {
    let owner = Owner::new();
    owner.set();
    let chat = chat_with_user(Uuid::new_v4());
    let (starts, stops) = install_inbox_retry_timer(chat);

    chat.finish_chat_inbox_snapshot_request(
        chat.inbox_request_generation
            .get_untracked()
            .saturating_add(1),
        chat.identity_untracked(),
        None,
    );

    assert_eq!(starts.load(Ordering::Relaxed), 0);
    assert_eq!(stops.load(Ordering::Relaxed), 0);
}

#[test]
fn inbox_success_and_session_clear_cancel_retry() {
    let owner = Owner::new();
    owner.set();
    let old_user_id = Uuid::new_v4();
    let chat = chat_with_user(old_user_id);
    let (starts, stops) = install_inbox_retry_timer(chat);
    let request_generation = chat.inbox_request_generation.get_untracked();
    let request_identity = chat.identity_untracked();

    chat.finish_chat_inbox_snapshot_request(request_generation, request_identity, None);
    chat.finish_chat_inbox_snapshot_request(
        request_generation,
        request_identity,
        Some(ChatInboxSnapshot::default()),
    );
    assert_eq!(starts.load(Ordering::Relaxed), 1);
    assert_eq!(stops.load(Ordering::Relaxed), 2);
    assert!(chat.inbox_ready.get_untracked());

    assert!(chat.apply_identity_change(
        Some(AuthIdentity::User(old_user_id)),
        Some(AuthIdentity::User(Uuid::new_v4())),
    ));
    assert_eq!(stops.load(Ordering::Relaxed), 3);
}
