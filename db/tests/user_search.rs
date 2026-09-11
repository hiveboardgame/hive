mod common;

use common::fixtures;
use db_lib::{get_conn, models::User, DbConn};

async fn search(pattern: &str, conn: &mut DbConn<'_>) -> Vec<String> {
    search_excluding(pattern, &[], conn).await
}

async fn search_excluding(pattern: &str, excluded: &[&str], conn: &mut DbConn<'_>) -> Vec<String> {
    let excluded: Vec<String> = excluded.iter().map(|name| (*name).to_string()).collect();
    User::search_usernames(pattern, &excluded, conn)
        .await
        .expect("search usernames")
        .into_iter()
        .map(|user| user.username)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_match_outranks_prefix_and_substring_matches() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for username in ["cupcakes", "iKessel", "Jakesyd", "KES", "kestrel"] {
        fixtures::create_user(username, false, &mut conn).await;
    }

    let ranked = search("KES", &mut conn).await;
    assert_eq!(
        ranked,
        vec!["KES", "kestrel", "cupcakes", "iKessel", "Jakesyd"]
    );
    let lowercase = search("kes", &mut conn).await;
    let uppercase = search("KES", &mut conn).await;
    assert_eq!(lowercase, uppercase);
}

#[tokio::test(flavor = "multi_thread")]
async fn wildcards_are_matched_literally() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for username in ["a_b", "axb", "a-b"] {
        fixtures::create_user(username, false, &mut conn).await;
    }

    assert_eq!(search("a_", &mut conn).await, vec!["a_b"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn deleted_users_are_excluded() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = fixtures::create_user("ghostly", false, &mut conn).await;
    user.soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    assert!(search("ghost", &mut conn).await.is_empty());
    assert!(search("deleted_user", &mut conn).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn excluded_users_do_not_consume_the_result_limit() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let names: Vec<String> = (0..26).map(|i| format!("team{i:02}")).collect();
    for username in &names {
        fixtures::create_user(username, false, &mut conn).await;
    }
    let invited: Vec<&str> = names[..25].iter().map(String::as_str).collect();

    assert_eq!(
        search_excluding("team", &invited, &mut conn).await,
        vec!["team25"],
    );
}
