mod common;

use common::tournament::{
    assert_well_formed,
    create_tournament,
    games_of,
    lower_seed_wins,
    play_unfinished,
    run_to_completion,
    standings_of,
    start,
    unordered,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome};
use hive_lib::Color;
use shared_types::{Tiebreaker, TournamentGameResult, TournamentMode};
use std::collections::{HashMap, HashSet};

/// Round one's matchups, as seeds, so the standard bracket shape can be
/// asserted directly.
fn seed_pairs(
    fixture: &common::tournament::Fixture,
    games: &[db_lib::models::Game],
    round: i32,
) -> Vec<(usize, usize)> {
    let mut pairs: HashSet<(usize, usize)> = HashSet::new();
    for game in games.iter().filter(|game| game.round == Some(round)) {
        let white = fixture.seed_of(game.white_id);
        let black = fixture.seed_of(game.black_id);
        pairs.insert((white.min(black), white.max(black)));
    }
    let mut pairs: Vec<(usize, usize)> = pairs.into_iter().collect();
    pairs.sort_unstable();
    pairs
}

#[tokio::test(flavor = "multi_thread")]
async fn single_elimination_seeds_the_bracket_and_plays_two_games_per_match() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 8,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;

    // Standard bracket seeding for eight: 1v8, 4v5, 2v7, 3v6 — zero-indexed
    // here, so 0v7, 3v4, 1v6, 2v5.
    assert_eq!(
        seed_pairs(&fixture, &first, 1),
        vec![(0, 7), (1, 6), (2, 5), (3, 4)],
    );
    assert_eq!(first.len(), 8, "four matches of two games each");

    let mut per_pair: HashMap<(uuid::Uuid, uuid::Uuid), Vec<&db_lib::models::Game>> =
        HashMap::new();
    for game in &first {
        per_pair
            .entry(unordered(game.white_id, game.black_id))
            .or_default()
            .push(game);
    }
    for (pair, games) in &per_pair {
        assert_eq!(games.len(), 2);
        assert_eq!(
            games.iter().filter(|game| game.white_id == pair.0).count(),
            1,
            "the two games of a match swap colours"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn single_elimination_with_a_third_place_match_ranks_the_whole_field() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 8,
            third_place_match: true,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 8);

    // With the better seed always winning: 0 beats 1 in the final, 2 beats 3
    // for bronze, and the four first-round losers are level.
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2], vec![3], vec![4, 5, 6, 7]],
    );
    let positions: Vec<u32> = standings
        .groups
        .iter()
        .map(|group| group[0].position)
        .collect();
    assert_eq!(
        positions,
        vec![1, 2, 3, 4, 5],
        "the four quarter-final losers all share fifth"
    );

    // Eight players is three rounds, so three is the most anybody can survive.
    // Counting finishing tiers instead would give the champion four here, since
    // the bronze match adds a tier without adding a round.
    let survived: Vec<f32> = standings
        .groups
        .iter()
        .map(|group| {
            *group[0]
                .scores
                .get(&Tiebreaker::RoundsSurvived)
                .expect("a bracket scores rounds survived")
        })
        .collect();
    assert_eq!(
        survived,
        vec![3.0, 2.0, 1.0, 1.0, 0.0],
        "the champion won all three rounds and the runner-up two; both bronze \
         players went out in the semi-final having won one, and the \
         quarter-final losers won none"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn single_elimination_without_a_third_place_match_ties_the_semi_finalists() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2, 3]],
        "with no bronze match the two beaten semi-finalists are level"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_non_power_of_two_field_gives_the_top_seeds_a_first_round_walkover() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 6,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;

    // An eight-slot bracket holding six players: the top two seeds advance
    // without playing, and the other four meet.
    assert_eq!(
        seed_pairs(&fixture, &first, 1),
        vec![(2, 5), (3, 4)],
        "only the middle four play a first round"
    );
    assert_eq!(first.len(), 4, "two matches of two games");

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 6);
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2, 3], vec![4, 5]],
    );
}

/// A bracket cannot stand on a draw, so unlike Double-Swiss a 1-1 attempt is
/// replayed rather than scored.
#[tokio::test(flavor = "multi_thread")]
async fn a_drawn_attempt_is_replayed_until_it_is_decisive() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;
    let split = unordered(first[0].white_id, first[0].black_id);

    // White wins both games of that match, which with swapped colours is 1-1.
    play_unfinished(
        &fixture,
        |game| {
            if unordered(game.white_id, game.black_id) == split {
                TournamentGameResult::Winner(Color::White)
            } else {
                let winner = if fixture.seed_of(game.white_id) < fixture.seed_of(game.black_id) {
                    Color::White
                } else {
                    Color::Black
                };
                TournamentGameResult::Winner(winner)
            }
        },
        &mut conn,
    )
    .await;

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("progress");
    let ProgressOutcome::Replays(replays) = outcome else {
        panic!("a drawn bracket match must be replayed, got {outcome:?}");
    };
    assert_eq!(replays.len(), 2);
    assert!(
        replays.iter().all(|game| game.round == Some(1)),
        "the replay belongs to the round that was drawn"
    );
    for game in &replays {
        assert_eq!(unordered(game.white_id, game.black_id), split);
    }

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(fixture.seed_groups(&standings)[0], vec![0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn double_elimination_gives_a_beaten_player_a_second_life() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        fixture.seed_groups(&standings)[0],
        vec![0],
        "the strongest player wins the bracket"
    );

    // Everyone has to lose twice, so a double-elimination event is longer than
    // the single-elimination version of the same field.
    let games = games_of(&fixture, &mut conn).await;
    let rounds: HashSet<i32> = games.iter().filter_map(|game| game.round).collect();
    assert!(
        rounds.len() >= 3,
        "a four-player double elimination needs at least three rounds, got {}",
        rounds.len()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn double_elimination_ranks_every_player() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleElimination,
        TournamentOpts {
            player_count: 8,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 8);
    assert_eq!(fixture.seed_groups(&standings)[0], vec![0]);

    let last = standings
        .groups
        .last()
        .expect("standings are never empty")
        .clone();
    assert!(
        last.iter()
            .all(|standing| fixture.seed_of(standing.player) >= 4),
        "the earliest exits should be the weakest players"
    );
}

/// A bracket has no withdrawal: the slot exists and somebody must come out of
/// it. So a player deleted between rounds is forfeited into their next match
/// rather than leaving it unplayable.
#[tokio::test(flavor = "multi_thread")]
async fn a_player_deleted_between_rounds_forfeits_their_next_match() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    // Round one plays out normally, top seeds through.
    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    // A semi-finalist deletes their account before the final is paired.
    let gone = fixture.players[1].id;
    let user = db_lib::models::User::find_by_uuid(&gone, &mut conn)
        .await
        .expect("load the leaving user");
    user.soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete");

    let outcome = fixture.tournament.progress(&mut conn).await.unwrap();
    let ProgressOutcome::Advanced(final_games) = outcome else {
        panic!("the final should still be paired, got {outcome:?}");
    };

    // The final exists and is already resolved, so the bracket is not stuck.
    assert!(
        final_games.iter().all(|game| game.finished),
        "a match against a deleted account is forfeited on creation"
    );
    assert!(
        matches!(
            fixture.tournament.progress(&mut conn).await.unwrap(),
            ProgressOutcome::ReadyToFinish
        ),
        "the bracket runs to completion despite the deletion"
    );

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        fixture.seed_groups(&standings)[0],
        vec![0],
        "the surviving finalist takes the title"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bulk_double_forfeit_is_refused_for_a_bracket() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    // A double forfeit scores as a drawn match, and a bracket cannot stand on
    // a draw — this would loop the round forever instead of clearing it.
    assert!(
        fixture
            .tournament
            .double_forfeit_unstarted_games(&fixture.organizer.id, &mut conn)
            .await
            .is_err(),
        "an organizer must adjudicate a winner in a bracket, not double-forfeit"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bracket_does_not_advance_while_a_match_is_unfinished() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleElimination,
        TournamentOpts {
            player_count: 4,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;

    // One game of one match is not a finished round.
    first[0]
        .adjudicate_tournament_result(
            &fixture.organizer.id,
            &TournamentGameResult::Winner(Color::White),
            &mut conn,
        )
        .await
        .expect("adjudicate one game");

    assert!(
        matches!(
            fixture.tournament.progress(&mut conn).await.unwrap(),
            ProgressOutcome::Waiting
        ),
        "the semi-finals are still being played"
    );
    assert_eq!(
        games_of(&fixture, &mut conn).await.len(),
        4,
        "no new games until the round is done"
    );
}
