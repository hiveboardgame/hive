use super::fixtures::*;

#[test]
fn different_identity_clears_session_state_exactly_once() {
    let owner = Owner::new();
    owner.set();

    let old_user_id = Uuid::new_v4();
    let new_user_id = Uuid::new_v4();
    let chat = Chat::new(test_websocket(), Some(AuthIdentity::User(old_user_id)));
    let key = ConversationKey::direct(Uuid::new_v4());
    let previous_epoch = chat.session_epoch_untracked();
    chat.conversation(key.clone())
        .draft()
        .set("old account draft".to_string());
    chat.add_local_unread(&key, 9);
    assert!(schedule_read(chat, &key, 10));
    assert_eq!(begin_scheduled_read(chat, &key), Some(10));
    chat.set_pending_read(&key, 10);
    let previous_catalog_refresh_epoch = chat.catalog_refresh_epoch.get_untracked();

    assert!(chat.apply_identity_change(
        Some(AuthIdentity::User(old_user_id)),
        Some(AuthIdentity::User(new_user_id)),
    ));

    assert_eq!(chat.session_epoch_untracked(), previous_epoch + 1);
    assert_eq!(
        chat.catalog_refresh_epoch.get_untracked(),
        previous_catalog_refresh_epoch,
    );
    assert!(chat
        .conversations
        .with_value(|registry| registry.entries.is_empty()));
    assert!(chat.unread.with_value(HashMap::is_empty));
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 0);
    let coordinator = chat.read_receipts.get_value();
    assert!(coordinator.scheduled_read_through.is_empty());
    assert!(coordinator.in_flight.is_empty());
    chat.finish_read_request(&key, 10, Some(10));
    assert_eq!(chat.read_floor_untracked(&key), 0);

    assert!(!chat.apply_identity_change(
        Some(AuthIdentity::User(new_user_id)),
        Some(AuthIdentity::User(new_user_id)),
    ));
    assert_eq!(chat.session_epoch_untracked(), previous_epoch + 1);
}
