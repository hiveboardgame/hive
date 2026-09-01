mod common;

use db_lib::{
    get_conn,
    models::{NewUser, User},
    schema::users,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::{TimeWarning, WarningTrigger, MAX_TIME_WARNINGS};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn never_configured_reads_as_none() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("warnings_unset", &mut conn).await;

    assert!(user.time_warning_stages().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn round_trips_and_caps_the_stage_count() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("warnings_round_trip", &mut conn).await;

    let requested = vec![
        TimeWarning::new(WarningTrigger::Proportional),
        TimeWarning::new(WarningTrigger::Remaining(30)),
        TimeWarning::new(WarningTrigger::Remaining(10)),
        TimeWarning::new(WarningTrigger::Remaining(5)),
    ];
    user.set_time_warnings(&requested, &mut conn)
        .await
        .expect("store time warnings");

    let stages = reload(user.id, &mut conn)
        .await
        .time_warning_stages()
        .expect("stored warnings parse");
    assert_eq!(stages.len(), MAX_TIME_WARNINGS);
    assert_eq!(stages, requested[..MAX_TIME_WARNINGS]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_explicitly_empty_list_is_not_the_same_as_unset() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("warnings_emptied", &mut conn).await;

    user.set_time_warnings(&[], &mut conn)
        .await
        .expect("store empty warnings");

    let stages = reload(user.id, &mut conn).await.time_warning_stages();
    assert_eq!(stages, Some(Vec::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_row_falls_back_instead_of_failing() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("warnings_malformed", &mut conn).await;

    diesel::update(users::table.find(user.id))
        .set(users::time_warnings.eq(Some(serde_json::json!({"at": "nonsense"}))))
        .execute(&mut conn)
        .await
        .expect("write malformed warnings");

    assert!(reload(user.id, &mut conn)
        .await
        .time_warning_stages()
        .is_none());
}

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn reload(id: Uuid, conn: &mut DbConn<'_>) -> User {
    User::find_active_by_uuid(&id, conn)
        .await
        .expect("reload user")
}
