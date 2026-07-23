mod common;

use db_lib::{
    get_conn,
    models::{NewUser, NotificationPreferences, NotificationPreferencesUpdate, User},
    DbConn,
};

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

fn update_with_general_chat(channels: Vec<Option<String>>) -> NotificationPreferencesUpdate {
    NotificationPreferencesUpdate {
        your_turn: vec![Some("push".to_string())],
        challenges: vec![Some("push".to_string())],
        game_ended: vec![Some("push".to_string())],
        tournament: vec![Some("push".to_string())],
        schedules: vec![Some("push".to_string())],
        general_chat: channels,
        dms: vec![Some("push".to_string())],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn update_for_user_round_trips_general_chat() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("general_chat_owner", &mut conn).await;
    NotificationPreferences::create_for_user(user.id, &mut conn)
        .await
        .expect("create default preferences");

    let updated = NotificationPreferences::update_for_user(
        user.id,
        update_with_general_chat(vec![Some("push".to_string())]),
        &mut conn,
    )
    .await
    .expect("update preferences");
    assert_eq!(updated.general_chat, vec![Some("push".to_string())]);

    let found = NotificationPreferences::find_for_user(user.id, &mut conn)
        .await
        .expect("find preferences");
    assert_eq!(found.general_chat, vec![Some("push".to_string())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn user_ids_with_general_chat_channel_only_returns_matching_users() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let opted_in = create_user("general_chat_opted_in", &mut conn).await;
    let opted_out = create_user("general_chat_opted_out", &mut conn).await;

    NotificationPreferences::create_for_user(opted_in.id, &mut conn)
        .await
        .expect("create default preferences");
    NotificationPreferences::create_for_user(opted_out.id, &mut conn)
        .await
        .expect("create default preferences");

    NotificationPreferences::update_for_user(
        opted_in.id,
        update_with_general_chat(vec![Some("push".to_string())]),
        &mut conn,
    )
    .await
    .expect("opt in to general chat push");
    // opted_out keeps the default '{}' general_chat value.

    let ids = NotificationPreferences::user_ids_with_general_chat_channel("push", &mut conn)
        .await
        .expect("query recipients");

    assert!(ids.contains(&opted_in.id));
    assert!(!ids.contains(&opted_out.id));
}
