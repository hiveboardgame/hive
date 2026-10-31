mod common;

use chrono::{DateTime, Utc};
use db_lib::{
    get_conn,
    models::HomeBanner,
    schema::home_banner::{self, content, display, title},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[tokio::test(flavor = "multi_thread")]
async fn update_stores_and_clears_event_time() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    diesel::insert_into(home_banner::table)
        .values((
            title.eq("Queen Blitz Arena"),
            content.eq("Play as many games as you can"),
            display.eq(true),
        ))
        .execute(&mut conn)
        .await
        .expect("seed banner");
    let event_time = "2026-11-07T18:00:00Z"
        .parse::<DateTime<Utc>>()
        .expect("event time");

    let mut banner = HomeBanner::get(&mut conn).await.expect("banner");
    assert_eq!(banner.starts_at, None);
    banner.starts_at = Some(event_time);
    banner
        .clone()
        .update(&mut conn)
        .await
        .expect("store event time");
    assert_eq!(
        HomeBanner::get(&mut conn).await.expect("banner").starts_at,
        Some(event_time)
    );

    banner.starts_at = None;
    banner.update(&mut conn).await.expect("clear event time");
    assert_eq!(
        HomeBanner::get(&mut conn).await.expect("banner").starts_at,
        None
    );
}
