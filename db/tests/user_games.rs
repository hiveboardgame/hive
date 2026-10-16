mod common;

use chrono::{Duration, Utc};
use common::fixtures::{create_game, create_user, new_bullet_game};
use db_lib::get_conn;

// A game past its deadline is lost on time and only waits for the sweeper, so neither "my
// games" nor "my turn" may offer it as playable.
#[tokio::test(flavor = "multi_thread")]
async fn games_past_their_deadline_are_not_listed_as_playable() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("test database connection");
    let white = create_user("deadline_white", false, &mut conn).await;
    let black = create_user("deadline_black", true, &mut conn).await;

    let live = create_game(new_bullet_game(white.id, black.id), &mut conn).await;
    let mut expired = new_bullet_game(white.id, black.id);
    expired.timeout_at = Some(Utc::now() - Duration::seconds(1));
    let expired = create_game(expired, &mut conn).await;

    for games in [
        white
            .get_games_with_notifications(&mut conn)
            .await
            .expect("pending games load"),
        white
            .get_ongoing_games(&mut conn)
            .await
            .expect("ongoing games load"),
        black
            .get_ongoing_games(&mut conn)
            .await
            .expect("ongoing games load"),
    ] {
        let ids: Vec<_> = games.iter().map(|game| game.id).collect();
        assert_eq!(ids, vec![live.id], "only the live game is playable");
        assert!(!ids.contains(&expired.id));
    }
}
