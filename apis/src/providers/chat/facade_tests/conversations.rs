use super::fixtures::*;

#[test]
fn offscreen_messages_update_only_their_lightweight_inbox_state() {
    let owner = Owner::new();
    owner.set();
    let current_user_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();

    let dm_chat = chat_with_user(current_user_id);
    let dm_key = ConversationKey::direct(sender_id);
    dm_chat.recv(ChatMessageContainer::new(
        dm_key.clone(),
        message(1, sender_id, "sender", "hello"),
        Uuid::new_v4(),
    ));
    assert!(dm_chat.conversation_if_exists(&dm_key).is_none());
    assert_eq!(dm_chat.unread_count_for_channel_untracked(&dm_key), 1);
    assert_eq!(
        dm_chat
            .catalog_activity()
            .map(|activity| (activity.key, activity.message_id)),
        Some((dm_key, 1)),
    );

    let player_chat = chat_with_user(current_user_id);
    let player_key = ConversationKey::game_players(&GameId("players".to_string()));
    player_chat.recv(ChatMessageContainer::new(
        player_key.clone(),
        message(2, sender_id, "opponent", "move chat"),
        Uuid::new_v4(),
    ));
    assert!(player_chat.conversation_if_exists(&player_key).is_none());
    assert_eq!(
        player_chat.unread_count_for_channel_untracked(&player_key),
        1
    );

    let spectator_chat = chat_with_user(current_user_id);
    let spectator_key = ConversationKey::game_spectators(&GameId("spectators".to_string()));
    spectator_chat.recv(ChatMessageContainer::new(
        spectator_key.clone(),
        message(3, sender_id, "spectator", "spectator chat"),
        Uuid::new_v4(),
    ));
    assert!(spectator_chat
        .conversation_if_exists(&spectator_key)
        .is_none());
    assert!(spectator_chat.catalog_activity().is_none());
}
