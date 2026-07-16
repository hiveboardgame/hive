use super::fixtures::*;

#[test]
fn muted_tournament_has_no_displayed_unread_count() {
    let owner = Owner::new();
    owner.set();

    let user_id = Uuid::new_v4();
    let tournament_id = TournamentId("muted-tournament".to_string());
    let key = ConversationKey::tournament(&tournament_id);
    let chat = chat_with_user(user_id);
    let refresh_epoch_before = chat.catalog_refresh_epoch.get_untracked();
    chat.apply_server_unread_states(vec![ConversationUnreadState {
        key: key.clone(),
        count: 4,
        latest_message_id: 12,
        latest_unread_message_id: 12,
        last_read_message_id: 4,
    }]);

    assert_eq!(chat.unread_count_for_channel_untracked(&key), 4);
    chat.set_tournament_muted(&tournament_id, true);
    assert_eq!(chat.unread_count_for_channel_untracked(&key), 0);
    assert_eq!(
        chat.catalog_refresh_epoch.get_untracked(),
        refresh_epoch_before
    );
    let refresh_epoch_after_update = chat.catalog_refresh_epoch.get_untracked();
    assert!(!chat.set_tournament_muted(&tournament_id, true));
    assert_eq!(
        chat.catalog_refresh_epoch.get_untracked(),
        refresh_epoch_after_update,
    );
    assert!(chat.tournament_muted_untracked(&tournament_id));
    assert!(chat.set_tournament_muted(&tournament_id, false));
}
