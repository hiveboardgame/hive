mod common;
use common::fixtures;
use db_lib::{
    get_conn,
    models::{BioModeration, PlayerProfile, User},
    schema::users,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

async fn admin(conn: &mut db_lib::DbConn<'_>) -> User {
    let actor = fixtures::create_user("profile_admin", false, conn).await;
    diesel::update(users::table.find(actor.id))
        .set(users::admin.eq(true))
        .execute(conn)
        .await
        .unwrap();
    actor
}

#[tokio::test(flavor = "multi_thread")]
async fn optional_profile_persists_and_clears_without_changing_another_account() {
    let db = common::db::test_db().await;
    let mut c = get_conn(&db.pool).await.unwrap();
    let a = fixtures::create_user("profile_a", false, &mut c).await;
    let b = fixtures::create_user("profile_b", false, &mut c).await;
    assert!(PlayerProfile::find(a.id, &mut c).await.unwrap().is_none());
    PlayerProfile::save_flag(a.id, Some("IT"), &mut c)
        .await
        .unwrap();
    PlayerProfile::save_bio(a.id, "🐝 Hello", &mut c)
        .await
        .unwrap();
    let p = PlayerProfile::find(a.id, &mut c).await.unwrap().unwrap();
    assert_eq!(p.bio, "🐝 Hello");
    assert_eq!(p.flag.as_deref(), Some("IT"));
    assert!(PlayerProfile::find(b.id, &mut c).await.unwrap().is_none());
    PlayerProfile::save_flag(a.id, None, &mut c).await.unwrap();
    PlayerProfile::save_bio(a.id, "", &mut c).await.unwrap();
    let p = PlayerProfile::find(a.id, &mut c).await.unwrap().unwrap();
    assert!(p.flag.is_none() && p.bio.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn restriction_clears_replacement_is_public_and_reinstatement_does_not_restore_content() {
    let db = common::db::test_db().await;
    let mut c = get_conn(&db.pool).await.unwrap();
    let actor = admin(&mut c).await;
    let owner = fixtures::create_user("profile_owner", false, &mut c).await;
    PlayerProfile::save_bio(owner.id, "Removed content", &mut c)
        .await
        .unwrap();
    PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Restrict, &mut c)
        .await
        .unwrap();
    let p = PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .unwrap();
    assert!(p.bio.is_empty() && p.bio_restricted);
    // Deliberately submit the old form after restriction has committed.
    PlayerProfile::save_bio(owner.id, "Removed content", &mut c)
        .await
        .unwrap();
    PlayerProfile::save_flag(owner.id, Some("GB"), &mut c)
        .await
        .unwrap();
    PlayerProfile::moderate_bio(
        actor.id,
        owner.id,
        BioModeration::Replace("Public replacement".into()),
        &mut c,
    )
    .await
    .unwrap();
    PlayerProfile::save_bio(owner.id, "Stale form", &mut c)
        .await
        .unwrap();
    let p = PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.bio, "Public replacement");
    assert!(p.bio_restricted);
    assert_eq!(p.flag.as_deref(), Some("GB"));
    PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Reinstate, &mut c)
        .await
        .unwrap();
    let p = PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.bio, "Public replacement");
    assert!(!p.bio_restricted);
    PlayerProfile::save_bio(owner.id, "Owner can edit again", &mut c)
        .await
        .unwrap();
    PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Clear, &mut c)
        .await
        .unwrap();
    assert!(PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .unwrap()
        .bio
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn moderation_checks_current_active_admin_authorization() {
    let db = common::db::test_db().await;
    let mut c = get_conn(&db.pool).await.unwrap();
    let actor = admin(&mut c).await;
    let owner = fixtures::create_user("profile_owner", false, &mut c).await;
    assert!(
        PlayerProfile::moderate_bio(owner.id, owner.id, BioModeration::Restrict, &mut c)
            .await
            .is_err()
    );
    diesel::update(users::table.find(actor.id))
        .set(users::admin.eq(false))
        .execute(&mut c)
        .await
        .unwrap();
    assert!(
        PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Restrict, &mut c)
            .await
            .is_err()
    );
    diesel::update(users::table.find(actor.id))
        .set((users::admin.eq(true), users::deleted.eq(true)))
        .execute(&mut c)
        .await
        .unwrap();
    assert!(
        PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Restrict, &mut c)
            .await
            .is_err()
    );
    assert!(PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_account_erases_profile_and_prevents_stale_writes() {
    let db = common::db::test_db().await;
    let mut c = get_conn(&db.pool).await.unwrap();
    let actor = admin(&mut c).await;
    let owner = fixtures::create_user("profile_deleted", false, &mut c).await;
    PlayerProfile::save_flag(owner.id, Some("IT"), &mut c)
        .await
        .unwrap();
    PlayerProfile::moderate_bio(actor.id, owner.id, BioModeration::Restrict, &mut c)
        .await
        .unwrap();
    owner.soft_delete("replacement-hash", &mut c).await.unwrap();
    assert!(PlayerProfile::find(owner.id, &mut c)
        .await
        .unwrap()
        .is_none());
    assert!(PlayerProfile::save_bio(owner.id, "stale", &mut c)
        .await
        .is_err());
    assert!(PlayerProfile::save_flag(owner.id, Some("GB"), &mut c)
        .await
        .is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn database_enforces_unicode_bio_length() {
    let db = common::db::test_db().await;
    let mut c = get_conn(&db.pool).await.unwrap();
    let owner = fixtures::create_user("profile_length", false, &mut c).await;
    PlayerProfile::save_bio(owner.id, &"🐝".repeat(300), &mut c)
        .await
        .unwrap();
    assert!(PlayerProfile::save_bio(owner.id, &"🐝".repeat(301), &mut c)
        .await
        .is_err());
    assert_eq!(
        PlayerProfile::find(owner.id, &mut c)
            .await
            .unwrap()
            .unwrap()
            .bio,
        "🐝".repeat(300)
    );
}
