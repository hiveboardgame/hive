use super::fixtures::*;

#[test]
fn reconnect_history_keeps_messages_received_during_refresh() {
    let owner = Owner::new();
    owner.set();
    let user_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let key = ConversationKey::direct(other_id);
    let chat = chat_with_user(user_id);
    let conversation = chat.conversation(key.clone());
    let request = chat.begin_initial_history_request(&conversation).unwrap();
    assert!(chat.apply_initial_history_result(
        &conversation,
        &request,
        Ok(ChatHistoryResponse::Page(history_page(
            1,
            50,
            other_id,
            None,
            Some(0),
        ))),
    ));

    chat.reset_history();
    chat.recv(ChatMessageContainer::new(
        ConversationKey::Direct(other_id),
        message(100, other_id, "peer", "arrived during refresh"),
        Uuid::new_v4(),
    ));
    let request = chat.begin_initial_history_request(&conversation).unwrap();
    assert!(chat.apply_initial_history_result(
        &conversation,
        &request,
        Ok(ChatHistoryResponse::Page(history_page(
            101,
            150,
            other_id,
            Some(101),
            Some(50),
        ))),
    ));

    let cached = conversation.messages().get_untracked();
    assert_eq!(cached.len(), 51);
    assert_eq!(cached.first().map(|message| message.id), Some(100));
}

#[test]
fn fresh_history_anchor_survives_a_stale_inbox_snapshot() {
    let owner = Owner::new();
    owner.set();
    let user_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let key = ConversationKey::direct(other_id);
    let chat = chat_with_user(user_id);
    let conversation = chat.conversation(key.clone());
    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 0,
        latest_message_id: 50,
        latest_unread_message_id: 0,
        last_read_message_id: 50,
    }]);
    let request = chat.begin_initial_history_request(&conversation).unwrap();
    assert!(chat.apply_initial_history_result(
        &conversation,
        &request,
        Ok(ChatHistoryResponse::Page(history_page(
            51,
            55,
            other_id,
            None,
            Some(2),
        ))),
    ));

    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key,
        count: 0,
        latest_message_id: 50,
        latest_unread_message_id: 0,
        last_read_message_id: 50,
    }]);

    assert!(matches!(
        conversation.initial().get_untracked(),
        InitialHistoryStatus::Ready {
            unread_anchor: Some(2),
            ..
        }
    ));
}
