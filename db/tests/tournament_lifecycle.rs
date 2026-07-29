mod common;

use common::tournament::{
    create_tournament,
    lower_seed_wins,
    play_unfinished,
    standings_of,
    start,
    TournamentOpts,
};
use db_lib::{
    get_conn,
    models::{ProgressOutcome, Tournament},
};
use shared_types::{TournamentMode, TournamentStatus};

/// Drives the `fully_automated` job loop the way the background ticker will:
/// play whatever is outstanding, then let `automatic_progress` decide.
///
/// Scoped to one tournament on purpose. `automatic_progress` scans every
/// automated tournament in the database, and the test binaries share one, so an
/// unscoped assertion here would see whatever another binary happens to be
/// running.
async fn tick(
    tournament: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Vec<(Tournament, ProgressOutcome)> {
    Tournament::automatic_progress(conn)
        .await
        .expect("automatic progress")
        .into_iter()
        .filter(|(progressed, _)| progressed.id == tournament)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fully_automated_swiss_advances_and_finishes_without_an_organizer() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 2,
            fully_automated: true,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;
    assert_eq!(first.len(), 2);

    // Nothing has been played, so the job leaves it alone.
    assert!(
        tick(fixture.tournament.id, &mut conn).await.is_empty(),
        "a tournament mid-round is not progressed"
    );

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    let progressed = tick(fixture.tournament.id, &mut conn).await;
    assert_eq!(progressed.len(), 1);
    assert!(
        matches!(progressed[0].1, ProgressOutcome::Advanced(_)),
        "round one is complete, so round two is paired"
    );

    let reloaded = Tournament::find(fixture.tournament.id, &mut conn)
        .await
        .expect("reload tournament");
    assert_eq!(reloaded.status, TournamentStatus::InProgress.to_string());

    play_unfinished(&fixture, &result, &mut conn).await;
    let progressed = tick(fixture.tournament.id, &mut conn).await;
    assert_eq!(progressed.len(), 1);
    assert!(matches!(progressed[0].1, ProgressOutcome::ReadyToFinish));
    assert_eq!(
        progressed[0].0.status,
        TournamentStatus::Finished.to_string(),
        "the job finishes the tournament itself"
    );

    // And it stops showing up once it is over.
    assert!(tick(fixture.tournament.id, &mut conn).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fully_automated_round_robin_finishes_once_every_game_is_played() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
        TournamentOpts {
            player_count: 3,
            fully_automated: true,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    assert!(tick(fixture.tournament.id, &mut conn).await.is_empty());

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, result, &mut conn).await;

    let progressed = tick(fixture.tournament.id, &mut conn).await;
    assert_eq!(progressed.len(), 1);
    assert!(
        matches!(progressed[0].1, ProgressOutcome::ReadyToFinish),
        "a round robin never advances rounds, it only ever ends"
    );
    assert_eq!(
        progressed[0].0.status,
        TournamentStatus::Finished.to_string()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_organizer_run_tournament_is_left_alone_by_the_job() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
        TournamentOpts {
            player_count: 3,
            fully_automated: false,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, result, &mut conn).await;

    assert!(
        tick(fixture.tournament.id, &mut conn).await.is_empty(),
        "the job must not touch an organizer-run tournament"
    );
    let reloaded = Tournament::find(fixture.tournament.id, &mut conn)
        .await
        .expect("reload tournament");
    assert_eq!(reloaded.status, TournamentStatus::InProgress.to_string());

    // The organizer can still drive it by hand.
    assert!(matches!(
        fixture
            .tournament
            .progress_by_organizer(&fixture.organizer.id, &mut conn)
            .await
            .expect("organizer progresses the tournament"),
        ProgressOutcome::ReadyToFinish
    ));
    let finished = common::tournament::finish(&fixture, &mut conn).await;
    assert_eq!(finished.status, TournamentStatus::Finished.to_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn standings_are_empty_before_a_tournament_starts() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let fixture = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert!(standings.groups.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_non_organizer_cannot_advance_the_tournament() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
        TournamentOpts {
            player_count: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let intruder = fixture.players[0].id;
    assert!(
        fixture
            .tournament
            .progress_by_organizer(&intruder, &mut conn)
            .await
            .is_err(),
        "a player is not an organizer"
    );
}
