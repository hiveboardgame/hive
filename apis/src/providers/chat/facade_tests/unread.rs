use super::fixtures::*;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn install_read_receipt_timer(
    chat: Chat,
    pending_initially: bool,
) -> (RwSignal<f64>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let delay_ms = RwSignal::new(if pending_initially {
        super::super::unread::READ_RECEIPT_FLUSH_DELAY.as_secs_f64() * 1_000.0
    } else {
        0.0
    });
    let pending = RwSignal::new(pending_initially);
    let starts = Arc::new(AtomicUsize::new(0));
    let stops = Arc::new(AtomicUsize::new(0));
    let starts_for_timer = Arc::clone(&starts);
    let stops_for_timer = Arc::clone(&stops);
    chat.read_receipt_timer
        .set_value(Some(super::super::TimeoutControls::new(
            delay_ms,
            pending.into(),
            move |_: ()| {
                starts_for_timer.fetch_add(1, Ordering::Relaxed);
                pending.set(true);
            },
            move || {
                stops_for_timer.fetch_add(1, Ordering::Relaxed);
                pending.set(false);
            },
        )));
    (delay_ms, starts, stops)
}

#[test]
fn old_visible_owner_cleanup_cannot_clear_same_key_replacement() {
    let owner = Owner::new();
    owner.set();
    let chat = chat_with_user(Uuid::new_v4());
    let key = ConversationKey::direct(Uuid::new_v4());

    let old_owner = chat.set_channel_visible(&key);
    let replacement_owner = chat.set_channel_visible(&key);
    chat.clear_channel_visible(old_owner);
    assert!(chat.is_channel_visible(&key));
    chat.clear_channel_visible(replacement_owner);
    assert!(!chat.is_channel_visible(&key));
}

#[test]
fn pending_read_blocks_stale_server_unread_snapshot() {
    let owner = Owner::new();
    owner.set();

    let user_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let chat = chat_with_user(user_id);
    let key = ConversationKey::direct(other_id);
    chat.set_pending_read(&key, 10);

    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 4,
        latest_message_id: 8,
        latest_unread_message_id: 8,
        last_read_message_id: 0,
    }]);

    assert_eq!(chat.unread_count_for_channel_untracked(&key), 0);
}

#[test]
fn server_snapshot_merges_local_unread_without_double_counting() {
    let owner = Owner::new();
    owner.set();

    let chat = chat_with_user(Uuid::new_v4());
    let key = ConversationKey::direct(Uuid::new_v4());
    chat.add_local_unread(&key, 11);
    chat.add_local_unread(&key, 13);

    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 2,
        latest_message_id: 11,
        latest_unread_message_id: 11,
        last_read_message_id: 5,
    }]);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 3);

    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 3,
        latest_message_id: 13,
        latest_unread_message_id: 13,
        last_read_message_id: 5,
    }]);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 3);
}

#[test]
fn receipt_coordinator_serializes_and_coalesces_per_conversation() {
    let owner = Owner::new();
    owner.set();

    let chat = chat_with_user(Uuid::new_v4());
    let key = ConversationKey::direct(Uuid::new_v4());
    assert!(schedule_read(chat, &key, 10));
    assert_eq!(begin_scheduled_read(chat, &key), Some(10));

    assert!(!schedule_read(chat, &key, 11));
    assert!(!schedule_read(chat, &key, 15));
    assert_eq!(begin_scheduled_read(chat, &key), None);
    let coordinator = chat.read_receipts.get_value();
    assert_eq!(coordinator.in_flight.get(&key), Some(&10));
    assert_eq!(coordinator.scheduled_read_through.get(&key), Some(&15));

    assert!(finish_in_flight(chat, &key, 10));
    record_confirmed_read(chat, &key, 10);
    assert_eq!(begin_scheduled_read(chat, &key), Some(15));
}

#[test]
fn failed_receipt_defers_retry_and_success_uses_authoritative_floor() {
    let owner = Owner::new();
    owner.set();

    let chat = chat_with_user(Uuid::new_v4());
    let (retry_delay_ms, retry_starts, _) = install_read_receipt_timer(chat, false);
    let refresh_epoch_before = chat.catalog_refresh_epoch.get_untracked();
    let key = ConversationKey::direct(Uuid::new_v4());
    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 2,
        latest_message_id: 10,
        latest_unread_message_id: 10,
        last_read_message_id: 0,
    }]);
    chat.add_local_unread(&key, 11);
    assert!(schedule_read(chat, &key, 11));
    assert_eq!(begin_scheduled_read(chat, &key), Some(11));
    chat.set_pending_read(&key, 11);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 0);

    chat.finish_read_request(&key, 11, None);

    assert_eq!(chat.read_floor_untracked(&key), 0);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 3);
    assert_eq!(
        chat.catalog_refresh_epoch.get_untracked(),
        refresh_epoch_before
    );
    assert!(chat.read_receipts.get_value().in_flight.is_empty());
    assert_eq!(
        chat.read_receipts
            .get_value()
            .scheduled_read_through
            .get(&key),
        Some(&11)
    );
    assert_eq!(retry_starts.load(Ordering::Relaxed), 1);
    assert_eq!(
        retry_delay_ms.get_untracked(),
        Duration::from_secs(10).as_secs_f64() * 1_000.0
    );

    assert_eq!(begin_scheduled_read(chat, &key), Some(11));
    chat.set_pending_read(&key, 11);
    chat.finish_read_request(&key, 11, Some(10));

    assert_eq!(chat.read_floor_untracked(&key), 10);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 1);
}

#[test]
fn failed_receipt_keeps_an_earlier_pending_flush() {
    let owner = Owner::new();
    owner.set();

    let chat = chat_with_user(Uuid::new_v4());
    let (delay_ms, starts, stops) = install_read_receipt_timer(chat, true);
    let key = ConversationKey::direct(Uuid::new_v4());
    assert!(schedule_read(chat, &key, 11));
    assert_eq!(begin_scheduled_read(chat, &key), Some(11));
    chat.set_pending_read(&key, 11);

    chat.finish_read_request(&key, 11, None);

    assert_eq!(starts.load(Ordering::Relaxed), 0);
    assert_eq!(stops.load(Ordering::Relaxed), 0);
    assert_eq!(
        delay_ms.get_untracked(),
        super::super::unread::READ_RECEIPT_FLUSH_DELAY.as_secs_f64() * 1_000.0
    );
    assert_eq!(
        chat.read_receipts
            .get_value()
            .scheduled_read_through
            .get(&key),
        Some(&11)
    );
}

#[test]
fn receiving_above_bottom_stays_unread_if_bottom_visibility_is_lost_before_flush() {
    let owner = Owner::new();
    owner.set();

    let current_user_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let chat = chat_with_user(current_user_id);
    let key = ConversationKey::direct(sender_id);
    let conversation = chat.conversation(key.clone());
    chat.merge_messages(
        &conversation,
        vec![message(19, sender_id, "sender", "already cached")],
    );
    let visible_owner = chat.set_channel_visible(&key);
    chat.recv(ChatMessageContainer::new(
        ConversationKey::Direct(sender_id),
        message(20, sender_id, "sender", "visible briefly"),
        Uuid::new_v4(),
    ));
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 1);
    assert!(chat
        .read_receipts
        .get_value()
        .scheduled_read_through
        .is_empty());

    let highest_cached_id =
        chat.latest_cached_message_id_untracked(&chat.conversation(key.clone()));
    assert_eq!(highest_cached_id, 20);
    chat.mark_thread_caught_up(&key, highest_cached_id);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 1);

    chat.clear_channel_visible(visible_owner);
    chat.flush_scheduled_reads();

    assert_eq!(chat.unread_count_for_channel_untracked(&key), 1);
    let coordinator = chat.read_receipts.get_value();
    assert!(coordinator.scheduled_read_through.is_empty());
}
