mod common;

use common::tournament::{
    assert_points_conserved,
    assert_well_formed,
    byes_of,
    create_tournament,
    games_of,
    lower_seed_wins,
    play_unfinished,
    run_to_completion,
    score,
    standings_of,
    start,
    unordered,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome};
use hive_lib::Color;
use shared_types::{Tiebreaker, TournamentGameResult, TournamentMode};
use std::collections::{HashMap, HashSet};

const SINGLE_GAME_SWISS: [TournamentMode; 2] =
    [TournamentMode::DutchSwiss, TournamentMode::BursteinSwiss];

#[tokio::test(flavor = "multi_thread")]
async fn single_game_swiss_pairs_one_game_per_round_and_never_repeats_an_opponent() {
    for mode in SINGLE_GAME_SWISS {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            mode,
            TournamentOpts {
                player_count: 8,
                rounds: 3,
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        let first = start(&mut fixture, &mut conn).await;
        assert_eq!(
            first.len(),
            4,
            "{mode} pairs eight players into four single games"
        );
        assert!(first.iter().all(|game| game.round == Some(1)));

        let result = lower_seed_wins(&fixture);
        let advances = run_to_completion(&fixture, result, &mut conn).await;
        assert_eq!(
            advances, 2,
            "{mode} runs three rounds, so it advances twice"
        );

        let games = games_of(&fixture, &mut conn).await;
        assert_eq!(games.len(), 12, "{mode}: three rounds of four games");

        let mut met: HashMap<(uuid::Uuid, uuid::Uuid), usize> = HashMap::new();
        for game in &games {
            *met.entry(unordered(game.white_id, game.black_id))
                .or_default() += 1;
        }
        assert!(
            met.values().all(|count| *count == 1),
            "{mode} must never pair the same two players twice"
        );

        for player in &fixture.players {
            let (white, black) = games.iter().fold((0i32, 0i32), |(white, black), game| {
                if game.white_id == player.id {
                    (white + 1, black)
                } else if game.black_id == player.id {
                    (white, black + 1)
                } else {
                    (white, black)
                }
            });
            assert_eq!(white + black, 3, "{mode}: everyone plays every round");
            assert!(
                (white - black).abs() <= 1,
                "{mode} left a player on {white} whites and {black} blacks"
            );
        }

        let standings = standings_of(&fixture, &mut conn).await;
        assert_well_formed(&standings, 8);
        assert_points_conserved(&standings, 12);

        // With strict dominance the top seed wins all three and is alone at
        // the top; the bottom seed can never win.
        assert_eq!(
            fixture.seed_groups(&standings)[0],
            vec![0],
            "{mode}: the strongest player is the outright winner"
        );
        assert_eq!(
            score(&standings, fixture.players[0].id, Tiebreaker::RawPoints),
            3.0
        );
        assert_eq!(
            score(&standings, fixture.players[7].id, Tiebreaker::RawPoints),
            0.0,
            "{mode}: the weakest player loses every game"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_plays_two_colour_swapped_games_per_pairing() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;
    assert_eq!(first.len(), 4, "two pairings, two games each");

    let mut per_pair: HashMap<(uuid::Uuid, uuid::Uuid), Vec<&db_lib::models::Game>> =
        HashMap::new();
    for game in &first {
        per_pair
            .entry(unordered(game.white_id, game.black_id))
            .or_default()
            .push(game);
    }
    assert_eq!(per_pair.len(), 2);
    for (pair, games) in &per_pair {
        assert_eq!(games.len(), 2, "a Double-Swiss match is two games");
        assert_eq!(
            games.iter().filter(|game| game.white_id == pair.0).count(),
            1,
            "the two games swap colours"
        );
        assert!(games.iter().all(|game| game.round == Some(1)));
    }

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(fixture.seed_groups(&standings)[0], vec![0]);
}

/// A split match is a legitimate drawn match under FIDE C.04.5 — it is worth
/// half to each side and the round still closes. Only a double forfeit forces a
/// replay, which is what the next test covers.
#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_split_match_scores_as_a_draw() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 1,
            scoring: shared_types::ScoringMode::Match,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    // Give white every game: each pairing's two games then split 1-1, because
    // the colours are swapped between them.
    play_unfinished(
        &fixture,
        |_| TournamentGameResult::Winner(Color::White),
        &mut conn,
    )
    .await;

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("progress");
    assert!(
        matches!(outcome, ProgressOutcome::ReadyToFinish),
        "a 1-1 match is a drawn match, not a replay: {outcome:?}"
    );

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        standings.groups.len(),
        1,
        "every match was drawn, so the whole field is level"
    );
    for player in &fixture.players {
        assert_eq!(
            score(&standings, player.id, Tiebreaker::RawPoints),
            0.5,
            "a drawn match is worth half a match point"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_double_forfeit_forces_a_replay_in_the_same_round() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 1,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;
    let forfeited = unordered(first[0].white_id, first[0].black_id);

    play_unfinished(
        &fixture,
        |game| {
            if unordered(game.white_id, game.black_id) == forfeited {
                TournamentGameResult::DoubeForfeit
            } else {
                TournamentGameResult::Winner(Color::White)
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
        panic!("a double-forfeited match must be replayed, got {outcome:?}");
    };
    assert_eq!(replays.len(), 2, "the pair plays a fresh two-game match");
    assert!(
        replays.iter().all(|game| game.round == Some(1)),
        "a replay stays inside the round it belongs to"
    );
    for game in &replays {
        assert_eq!(unordered(game.white_id, game.black_id), forfeited);
    }

    // And once the replay is decisive the round closes.
    play_unfinished(
        &fixture,
        |_| TournamentGameResult::Winner(Color::White),
        &mut conn,
    )
    .await;
    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("progress");
    assert!(
        matches!(outcome, ProgressOutcome::ReadyToFinish),
        "got {outcome:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_odd_swiss_field_gives_one_bye_per_round_worth_a_full_point() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 5,
            rounds: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let first = start(&mut fixture, &mut conn).await;
    assert_eq!(first.len(), 2, "five players means two games and one bye");

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let byes = byes_of(&fixture, &mut conn).await;
    assert_eq!(byes.len(), 3, "one bye in each of the three rounds");
    let mut per_round: HashMap<i32, usize> = HashMap::new();
    let mut per_player: HashMap<uuid::Uuid, usize> = HashMap::new();
    for bye in &byes {
        *per_round.entry(bye.round).or_default() += 1;
        *per_player.entry(bye.user_id).or_default() += 1;
    }
    assert!(per_round.values().all(|count| *count == 1));
    assert!(
        per_player.values().all(|count| *count == 1),
        "nobody should be given two byes while others have none"
    );

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 5);

    // Six games were played and three byes handed out, each worth a full
    // point, so the field's total is nine.
    assert_points_conserved(&standings, 9);
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_pairings_are_reproducible_from_the_seeds() {
    let mut runs = Vec::new();
    for _ in 0..2 {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            TournamentMode::DutchSwiss,
            TournamentOpts {
                player_count: 8,
                rounds: 3,
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        start(&mut fixture, &mut conn).await;
        let result = lower_seed_wins(&fixture);
        run_to_completion(&fixture, result, &mut conn).await;

        let games = games_of(&fixture, &mut conn).await;
        let mut shape: Vec<(i32, usize, usize)> = games
            .iter()
            .map(|game| {
                (
                    game.round.expect("round is recorded"),
                    fixture.seed_of(game.white_id),
                    fixture.seed_of(game.black_id),
                )
            })
            .collect();
        shape.sort_unstable();

        let standings = standings_of(&fixture, &mut conn).await;
        runs.push((shape, fixture.seed_groups(&standings)));
    }

    // The old implementation shuffled the field with a thread RNG, so this
    // could not have held before.
    assert_eq!(
        runs[0], runs[1],
        "identical ratings and results must produce identical pairings and standings"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_round_is_paired_before_the_next_one_is_created() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 8,
            rounds: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    for round in 1..=3 {
        let games = games_of(&fixture, &mut conn).await;
        let rounds: HashSet<i32> = games.iter().filter_map(|game| game.round).collect();
        assert_eq!(
            rounds.len(),
            round as usize,
            "round {round} must not exist before its predecessor is done"
        );

        // Half-finishing a round is not enough to advance.
        let unfinished: Vec<&db_lib::models::Game> =
            games.iter().filter(|game| !game.finished).collect();
        unfinished[0]
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
            "the round is not over yet"
        );

        let result = lower_seed_wins(&fixture);
        play_unfinished(&fixture, result, &mut conn).await;
        let outcome = fixture.tournament.progress(&mut conn).await.unwrap();
        if round < 3 {
            assert!(
                matches!(outcome, ProgressOutcome::Advanced(_)),
                "round {round} should advance, got {outcome:?}"
            );
        } else {
            assert!(
                matches!(outcome, ProgressOutcome::ReadyToFinish),
                "the last round should end the tournament, got {outcome:?}"
            );
        }
    }
}

/// Neither player turned up, so neither scores. tournamint routes an unplayed
/// draw to `forfeit_loss` rather than the draw score, which is what lets a
/// tournament set what a no-show is worth — zero here, hive's long-standing
/// behaviour and the default.
#[tokio::test(flavor = "multi_thread")]
async fn a_double_forfeit_is_worth_nothing_to_either_player() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 1,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;
    play_unfinished(&fixture, |_| TournamentGameResult::DoubeForfeit, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    for player in &fixture.players {
        assert_eq!(
            score(&standings, player.id, Tiebreaker::RawPoints),
            0.0,
            "a no-show earns the forfeit score, which defaults to nothing"
        );
    }
}

/// A double forfeit scores as a drawn match, and Double-Swiss cannot stand on
/// one any more than a bracket can: the pairing comes straight back as a replay
/// instead of the round closing. The bulk button has to refuse it.
#[tokio::test(flavor = "multi_thread")]
async fn a_double_swiss_round_cannot_be_cleared_by_double_forfeiting_it() {
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

    assert!(
        fixture
            .tournament
            .double_forfeit_unstarted_games(&fixture.organizer.id, &mut conn)
            .await
            .is_err(),
        "an organizer must adjudicate a winner in a two-game match"
    );
}

/// A Swiss cannot pair more rounds than it has players — everyone runs out of
/// legal opponents, and Dutch's no-rematch criterion is absolute, so
/// `pair_next_round` fails outright and the tournament is stuck with no
/// organizer action that clears it.
#[tokio::test(flavor = "multi_thread")]
async fn a_swiss_with_more_rounds_than_its_smallest_legal_field_is_refused() {
    // Sixteen seats but only four needed to start. Checking the round count
    // against `seats` would wave this through and leave a four-player field
    // trying to play four rounds.
    let details = common::tournament::details_for(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 4,
            seats: Some(16),
            ..Default::default()
        },
    );
    assert!(
        db_lib::models::NewTournament::new(details).is_err(),
        "the round count has to fit the field that can actually start, not the seat ceiling"
    );

    // One fewer round does fit.
    let details = common::tournament::details_for(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 3,
            seats: Some(16),
            ..Default::default()
        },
    );
    assert!(db_lib::models::NewTournament::new(details).is_ok());
}

/// The creation guard makes this unreachable for anything created after it, but
/// tournaments predating it were validated against the seat ceiling and can
/// already be in the database with more rounds than players. `start` is the
/// first point the real field size is known, so it is the backstop.
#[tokio::test(flavor = "multi_thread")]
async fn starting_a_swiss_that_cannot_pair_all_its_rounds_is_refused() {
    use db_lib::schema::tournaments;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;

    // Exactly the shape a tournament created under the old validation can have.
    diesel::update(tournaments::table.find(fixture.tournament.id))
        .set(tournaments::rounds.eq(4))
        .execute(&mut conn)
        .await
        .expect("age the tournament into the old shape");
    let legacy = db_lib::models::Tournament::find(fixture.tournament.id, &mut conn)
        .await
        .expect("reload tournament");

    assert!(
        legacy.start(&mut conn).await.is_err(),
        "four players cannot play four rounds, and finding out mid-event is unrecoverable"
    );
}
