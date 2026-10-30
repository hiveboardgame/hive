mod common;

use common::fixtures;
use db_lib::{get_conn, models::PlayerProfile};

#[tokio::test(flavor = "multi_thread")]
async fn profile_updates_clear_nullable_fields_and_keep_users_separate() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let alice = fixtures::create_user("profile_alice", false, &mut conn).await;
    let bob = fixtures::create_user("profile_bob", false, &mut conn).await;
    assert!(PlayerProfile::find(alice.id, &mut conn)
        .await
        .unwrap()
        .is_none());
    let mut profile = PlayerProfile {
        user_id: alice.id,
        flag: Some("GB".into()),
        bio: "Hello 🐝".into(),
    };
    profile.save(&mut conn).await.unwrap();
    assert!(PlayerProfile::find(bob.id, &mut conn)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        PlayerProfile::find(alice.id, &mut conn)
            .await
            .unwrap()
            .unwrap()
            .bio,
        "Hello 🐝"
    );
    profile.flag = None;
    profile.bio.clear();
    profile.save(&mut conn).await.unwrap();
    let saved = PlayerProfile::find(alice.id, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(saved.flag.is_none() && saved.bio.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_account_erases_personal_profile() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let user = fixtures::create_user("profile_deleted", false, &mut conn).await;
    PlayerProfile {
        user_id: user.id,
        flag: None,
        bio: "Erase me".into(),
    }
    .save(&mut conn)
    .await
    .unwrap();
    user.soft_delete("replacement-password-hash", &mut conn)
        .await
        .unwrap();
    assert!(PlayerProfile::find(user.id, &mut conn)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn profile_schema_does_not_store_demographics() {
    use diesel::{sql_types::Text, QueryableByName};
    use diesel_async::RunQueryDsl;
    #[derive(QueryableByName)]
    struct Column {
        #[diesel(sql_type = Text)]
        column_name: String,
    }
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let columns = diesel::sql_query("SELECT column_name FROM information_schema.columns WHERE table_schema='public' AND table_name='player_profiles' ORDER BY ordinal_position")
        .load::<Column>(&mut conn).await.unwrap().into_iter().map(|row| row.column_name).collect::<Vec<_>>();
    assert_eq!(columns, vec!["user_id", "flag", "bio"]);
}
