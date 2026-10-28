mod common;

use common::fixtures::create_user;
use db_lib::{get_conn, models::TutorialProgress};

fn ids(lessons: &[&str]) -> Vec<String> {
    lessons.iter().map(|lesson| lesson.to_string()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn completing_lessons_is_idempotent() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("tutorial_learner", false, &mut conn).await;

    let inserted =
        TutorialProgress::complete(user.id, &ids(&["bugs.ant", "basics.welcome"]), &mut conn)
            .await
            .expect("complete lessons");
    assert_eq!(inserted, 2);

    let inserted =
        TutorialProgress::complete(user.id, &ids(&["bugs.ant", "bugs.queen"]), &mut conn)
            .await
            .expect("complete lessons again");
    assert_eq!(inserted, 1);

    let completed = TutorialProgress::completed_lessons(user.id, &mut conn)
        .await
        .expect("load progress");
    assert_eq!(
        completed,
        ids(&["basics.welcome", "bugs.ant", "bugs.queen"])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_is_per_user() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let first = create_user("tutorial_first", false, &mut conn).await;
    let second = create_user("tutorial_second", false, &mut conn).await;

    TutorialProgress::complete(first.id, &ids(&["bugs.beetle"]), &mut conn)
        .await
        .expect("complete lesson");

    let completed = TutorialProgress::completed_lessons(second.id, &mut conn)
        .await
        .expect("load progress");
    assert!(completed.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn completing_nothing_writes_nothing() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("tutorial_idle", false, &mut conn).await;

    let inserted = TutorialProgress::complete(user.id, &[], &mut conn)
        .await
        .expect("empty completion");
    assert_eq!(inserted, 0);
}
