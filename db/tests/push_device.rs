mod common;

use chrono::{Duration, Utc};
use db_lib::{
    get_conn,
    models::{NewPushDevice, NewUser, PushDevice, User},
    schema::push_devices,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn revoke_tombstones_device_and_hides_it_from_find() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("revoke_owner", &mut conn).await;

    let device = PushDevice::upsert(new_device(user.id, "endpoint-a"), true, &mut conn)
        .await
        .expect("register device");
    assert_eq!(find_ids(user.id, &mut conn).await, vec![device.id]);

    PushDevice::revoke_for_user(device.id, user.id, &mut conn)
        .await
        .expect("revoke device");

    assert!(find_ids(user.id, &mut conn).await.is_empty());
    let rows = all_rows(user.id, &mut conn).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].revoked_at.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn reconcile_does_not_resurrect_revoked_device() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("reconcile_owner", &mut conn).await;

    let device = PushDevice::upsert(new_device(user.id, "endpoint-b"), true, &mut conn)
        .await
        .expect("register device");
    PushDevice::revoke_for_user(device.id, user.id, &mut conn)
        .await
        .expect("revoke device");

    PushDevice::upsert(new_device(user.id, "endpoint-b"), false, &mut conn)
        .await
        .expect("reconcile upsert");

    assert!(find_ids(user.id, &mut conn).await.is_empty());
    let rows = all_rows(user.id, &mut conn).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].revoked_at.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn explicit_subscribe_clears_tombstone() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("reenable_owner", &mut conn).await;

    let device = PushDevice::upsert(new_device(user.id, "endpoint-c"), true, &mut conn)
        .await
        .expect("register device");
    PushDevice::revoke_for_user(device.id, user.id, &mut conn)
        .await
        .expect("revoke device");
    assert!(find_ids(user.id, &mut conn).await.is_empty());

    PushDevice::upsert(new_device(user.id, "endpoint-c"), true, &mut conn)
        .await
        .expect("explicit re-subscribe");

    assert_eq!(find_ids(user.id, &mut conn).await, vec![device.id]);
    let rows = all_rows(user.id, &mut conn).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].revoked_at.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn reconcile_restores_swept_device() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("swept_owner", &mut conn).await;

    PushDevice::upsert(new_device(user.id, "endpoint-d"), true, &mut conn)
        .await
        .expect("register device");

    let removed = PushDevice::delete_stale(Utc::now() + Duration::days(1), &mut conn)
        .await
        .expect("run stale sweep");
    assert_eq!(removed, 1);
    assert!(all_rows(user.id, &mut conn).await.is_empty());

    PushDevice::upsert(new_device(user.id, "endpoint-d"), false, &mut conn)
        .await
        .expect("reconcile upsert");

    let rows = all_rows(user.id, &mut conn).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].revoked_at.is_none());
    assert_eq!(find_ids(user.id, &mut conn).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_stale_skips_tombstones() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("sweep_owner", &mut conn).await;

    PushDevice::upsert(new_device(user.id, "endpoint-active"), true, &mut conn)
        .await
        .expect("register active device");
    let revoked = PushDevice::upsert(new_device(user.id, "endpoint-revoked"), true, &mut conn)
        .await
        .expect("register device to revoke");
    PushDevice::revoke_for_user(revoked.id, user.id, &mut conn)
        .await
        .expect("revoke device");

    let removed = PushDevice::delete_stale(Utc::now() + Duration::days(1), &mut conn)
        .await
        .expect("run stale sweep");
    assert_eq!(removed, 1);

    let rows = all_rows(user.id, &mut conn).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, revoked.id);
    assert!(rows[0].revoked_at.is_some());
}

fn new_device(user_id: Uuid, token: &str) -> NewPushDevice {
    NewPushDevice {
        user_id,
        platform: "web".to_string(),
        device_token: token.to_string(),
        app_version: "test".to_string(),
        locale: "en".to_string(),
        p256dh: Some("p256dh-key".to_string()),
        auth: Some("auth-key".to_string()),
    }
}

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn find_ids(uid: Uuid, conn: &mut DbConn<'_>) -> Vec<Uuid> {
    PushDevice::find_for_user(uid, conn)
        .await
        .expect("find_for_user")
        .into_iter()
        .map(|d| d.id)
        .collect()
}

async fn all_rows(uid: Uuid, conn: &mut DbConn<'_>) -> Vec<PushDevice> {
    push_devices::table
        .filter(push_devices::user_id.eq(uid))
        .select(PushDevice::as_select())
        .load(conn)
        .await
        .expect("load all push devices")
}
