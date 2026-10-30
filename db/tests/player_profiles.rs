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
        bio_blocked: false,
    };
    profile.save_owner(&mut conn).await.unwrap();
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
    profile.save_owner(&mut conn).await.unwrap();
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
        bio_blocked: false,
    }
    .save_owner(&mut conn)
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
    assert_eq!(columns, vec!["user_id", "flag", "bio", "bio_blocked"]);
}

async fn make_admin(conn: &mut db_lib::DbConn<'_>) -> db_lib::models::User {
    use diesel::ExpressionMethods;
    use diesel_async::RunQueryDsl;
    let admin = fixtures::create_user("bio_admin", false, conn).await;
    diesel::update(db_lib::schema::users::table)
        .filter(db_lib::schema::users::id.eq(admin.id))
        .set(db_lib::schema::users::admin.eq(true))
        .execute(conn)
        .await
        .unwrap();
    admin
}

#[tokio::test(flavor = "multi_thread")]
async fn moderation_requires_a_current_active_admin() {
    use db_lib::{db_error::DbError, schema::users};
    use diesel::{ExpressionMethods, QueryDsl};
    use diesel_async::RunQueryDsl;
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let owner = fixtures::create_user("bio_owner", false, &mut conn).await;
    let other = fixtures::create_user("bio_other", false, &mut conn).await;
    let admin = make_admin(&mut conn).await;
    for actor in [owner.id, other.id] {
        assert!(matches!(
            PlayerProfile::moderate_bio(actor, owner.id, "Changed".into(), true, &mut conn).await,
            Err(DbError::Unauthorized)
        ));
        assert!(PlayerProfile::find(owner.id, &mut conn)
            .await
            .unwrap()
            .is_none());
    }
    diesel::update(users::table.filter(users::id.eq(admin.id)))
        .set(users::admin.eq(false))
        .execute(&mut conn)
        .await
        .unwrap();
    assert!(matches!(
        PlayerProfile::moderate_bio(admin.id, owner.id, "Changed".into(), true, &mut conn).await,
        Err(DbError::Unauthorized)
    ));
    diesel::update(users::table.filter(users::id.eq(admin.id)))
        .set((users::admin.eq(true), users::deleted.eq(true)))
        .execute(&mut conn)
        .await
        .unwrap();
    assert!(
        PlayerProfile::moderate_bio(admin.id, owner.id, "Changed".into(), true, &mut conn)
            .await
            .is_err()
    );
    assert!(PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_can_edit_block_and_unblock_without_changing_the_flag() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let owner = fixtures::create_user("bio_owner", false, &mut conn).await;
    let admin = make_admin(&mut conn).await;
    let mut profile = PlayerProfile {
        user_id: owner.id,
        flag: Some("GB".into()),
        bio: "Original".into(),
        bio_blocked: false,
    };
    profile.save_owner(&mut conn).await.unwrap();
    PlayerProfile::moderate_bio(
        admin.id,
        owner.id,
        "Removed by an administrator".into(),
        true,
        &mut conn,
    )
    .await
    .unwrap();
    // A stale or forged owner submission must not overwrite the bio or unlock it.
    profile.flag = Some("IT".into());
    profile.bio = "Attempted replacement".into();
    profile.bio_blocked = false;
    profile.save_owner(&mut conn).await.unwrap();
    let blocked = PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(blocked.bio_blocked);
    assert_eq!(blocked.bio, "Removed by an administrator");
    assert_eq!(blocked.flag.as_deref(), Some("IT"));
    PlayerProfile::moderate_bio(admin.id, owner.id, "Approved text".into(), false, &mut conn)
        .await
        .unwrap();
    let approved = PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(!approved.bio_blocked);
    assert_eq!(approved.bio, "Approved text");
    assert_eq!(approved.flag.as_deref(), Some("IT"));
    profile.bio = "Owner can edit again".into();
    profile.save_owner(&mut conn).await.unwrap();
    assert_eq!(
        PlayerProfile::find(owner.id, &mut conn)
            .await
            .unwrap()
            .unwrap()
            .bio,
        "Owner can edit again"
    );
    PlayerProfile::moderate_bio(admin.id, owner.id, String::new(), false, &mut conn)
        .await
        .unwrap();
    assert!(PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .bio
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_owner_edits_cannot_undo_a_moderation_block() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let owner = fixtures::create_user("bio_owner", false, &mut conn).await;
    let admin = make_admin(&mut conn).await;
    for _ in 0..8 {
        PlayerProfile::moderate_bio(admin.id, owner.id, "Initial".into(), false, &mut conn)
            .await
            .unwrap();
        let mut owner_conn = get_conn(&db.pool).await.unwrap();
        let mut admin_conn = get_conn(&db.pool).await.unwrap();
        let profile = PlayerProfile {
            user_id: owner.id,
            flag: Some("FR".into()),
            bio: "Concurrent owner edit".into(),
            bio_blocked: false,
        };
        let (owner_write, admin_write) = tokio::join!(
            profile.save_owner(&mut owner_conn),
            PlayerProfile::moderate_bio(
                admin.id,
                owner.id,
                "Moderator text".into(),
                true,
                &mut admin_conn
            ),
        );
        owner_write.unwrap();
        admin_write.unwrap();
        let result = PlayerProfile::find(owner.id, &mut conn)
            .await
            .unwrap()
            .unwrap();
        assert!(result.bio_blocked);
        assert_eq!(result.bio, "Moderator text");
        assert_eq!(result.flag.as_deref(), Some("FR"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn moderation_checks_target_and_unicode_length_before_writing() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let owner = fixtures::create_user("bio_owner", false, &mut conn).await;
    let admin = make_admin(&mut conn).await;
    assert!(PlayerProfile::moderate_bio(
        admin.id,
        uuid::Uuid::new_v4(),
        "Missing target".into(),
        true,
        &mut conn
    )
    .await
    .is_err());
    assert!(
        PlayerProfile::moderate_bio(admin.id, owner.id, "🐝".repeat(301), true, &mut conn)
            .await
            .is_err()
    );
    assert!(PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .is_none());
    PlayerProfile::moderate_bio(admin.id, owner.id, "🐝".repeat(300), true, &mut conn)
        .await
        .unwrap();
    owner
        .soft_delete("deleted-password-hash", &mut conn)
        .await
        .unwrap();
    assert!(PlayerProfile::find(owner.id, &mut conn)
        .await
        .unwrap()
        .is_none());
    assert!(PlayerProfile::moderate_bio(
        admin.id,
        owner.id,
        "Deleted target".into(),
        true,
        &mut conn
    )
    .await
    .is_err());
}
