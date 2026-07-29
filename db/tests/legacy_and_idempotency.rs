mod common;

use common::tournament::{
    create_tournament,
    games_of,
    lower_seed_wins,
    play_unfinished,
    standings_of,
    start,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome, schema::tournaments_users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::TournamentMode;

/// Rows written before the `seed` and `rating` columns existed have neither.
/// Replay has to cope, because every tournament in the database predates them
/// and `standings()` replays a tournament on every single request.
#[tokio::test(flavor = "multi_thread")]
async fn a_tournament_whose_players_predate_the_seed_column_still_has_standings() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    // Exactly what a pre-migration row looks like.
    diesel::update(
        tournaments_users::table.filter(tournaments_users::tournament_id.eq(fixture.tournament.id)),
    )
    .set((
        tournaments_users::seed.eq(None::<i32>),
        tournaments_users::rating.eq(None::<f64>),
    ))
    .execute(&mut conn)
    .await
    .expect("strip the seeds");

    let standings = standings_of(&fixture, &mut conn).await;
    assert_eq!(
        standings
            .groups
            .iter()
            .map(|group| group.len())
            .sum::<usize>(),
        4,
        "every player is still ranked once the seeds are derived"
    );
}

/// A game with no round is a hard error for every format now — including round
/// robin, which used to skip such games silently and report an all-zero table.
#[tokio::test(flavor = "multi_thread")]
async fn a_round_robin_game_without_a_round_is_refused_rather_than_skipped() {
    use db_lib::schema::games;

    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    diesel::update(games::table.filter(games::tournament_id.eq(Some(fixture.tournament.id))))
        .set(games::round.eq(None::<i32>))
        .execute(&mut conn)
        .await
        .expect("strip the rounds");

    assert!(
        fixture.tournament.standings(&mut conn).await.is_err(),
        "scoring nothing at all is worse than saying the backfill has not been run"
    );
}

/// The replay games a tick creates are unfinished, so the pair that needed them
/// is invisible to the next replay. Without an explicit guard every later tick
/// creates another match for it, forever.
#[tokio::test(flavor = "multi_thread")]
async fn a_pending_replay_is_not_created_twice() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 2,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    // Both games of every match forfeited. A *drawn* match is a legitimate
    // result and just advances; only a match where neither game was played
    // comes back as `NeedsReplay` (FIDE C.04.5, `double_swiss_match_outcome`).
    play_unfinished(
        &fixture,
        |_| shared_types::TournamentGameResult::DoubeForfeit,
        &mut conn,
    )
    .await;

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("progress");
    let ProgressOutcome::Replays(first) = outcome else {
        panic!("a drawn double-swiss match has to be replayed, got {outcome:?}");
    };
    assert!(!first.is_empty());
    let after_first = games_of(&fixture, &mut conn).await.len();

    // The replays are still unfinished. Ticking again must do nothing.
    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("progress again");
    assert!(
        matches!(outcome, ProgressOutcome::Waiting),
        "a replay is already outstanding, so the tick waits: {outcome:?}"
    );
    assert_eq!(
        games_of(&fixture, &mut conn).await.len(),
        after_first,
        "ticking twice must not create a second set of replay games"
    );
}
