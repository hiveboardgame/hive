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
