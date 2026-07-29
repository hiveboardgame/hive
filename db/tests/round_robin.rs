mod common;

use common::tournament::{
    assert_points_conserved,
    assert_well_formed,
    byes_of,
    create_tournament,
    games_of,
    lower_seed_wins,
    meetings,
    play_unfinished,
    run_to_completion,
    score,
    seeds,
    standings_of,
    start,
    unordered,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome};
use hive_lib::Color;
use shared_types::{Tiebreaker, TournamentGameResult, TournamentMode};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

/// N players need N-1 rounds per pass when the field is even, and N when it is
/// odd (one player sits out each round).
fn expected_rounds(players: usize, repeats: usize) -> usize {
    if players.is_multiple_of(2) {
        (players - 1) * repeats
    } else {
        players * repeats
    }
}

/// A seeded multiplicative hash rather than an RNG: the mix of wins, draws and
/// losses stays varied across pairings but is identical on every run, so a
/// failure is always reproducible.
fn scripted_result(round: i32, white_seed: usize, black_seed: usize) -> TournamentGameResult {
    let seed = (round as u64)
        .wrapping_mul(1_000)
        .wrapping_add(white_seed as u64 * 100)
        .wrapping_add(black_seed as u64);
    match seed.wrapping_mul(2_654_435_761) % 3 {
        0 => TournamentGameResult::Winner(Color::White),
        1 => TournamentGameResult::Draw,
        _ => TournamentGameResult::Winner(Color::Black),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn single_round_robin_final_standings_are_exactly_seed_order() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 6,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 6);

    // Strict dominance means no ties at all: seed 0 beat everyone, seed 1 beat
    // everyone but seed 0, and so on down.
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2], vec![3], vec![4], vec![5]],
    );
    for (index, player) in fixture.players.iter().enumerate() {
        assert_eq!(
            score(&standings, player.id, Tiebreaker::RawPoints),
            (5 - index) as f32,
            "seed {index} beats every lower seed and loses to every higher one"
        );
    }
    assert_points_conserved(&standings, 15);
}

#[tokio::test(flavor = "multi_thread")]
async fn round_robin_schedules_every_pair_the_right_number_of_times() {
    for (mode, players, repeats) in [
        (TournamentMode::SingleRoundRobin, 6, 1),
        (TournamentMode::DoubleRoundRobin, 5, 2),
        (TournamentMode::QuadrupleRoundRobin, 4, 4),
        (TournamentMode::SextupleRoundRobin, 4, 6),
    ] {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            mode,
            TournamentOpts {
                player_count: players,
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        let games = start(&mut fixture, &mut conn).await;

        let pairs = players * (players - 1) / 2;
        assert_eq!(
            games.len(),
            pairs * repeats,
            "{mode} with {players} players must schedule every pair {repeats} times"
        );

        let (met, whites) = meetings(&games);
        assert_eq!(met.len(), pairs, "{mode} must pair every possible couple");
        for count in met.values() {
            assert_eq!(*count, repeats, "{mode} pairs each couple {repeats} times");
        }
        // An even number of passes splits every pair's colours exactly; an odd
        // number cannot, and leaves each pair one game out of balance.
        for count in whites.values() {
            assert!(
                *count == repeats / 2 || *count == repeats.div_ceil(2),
                "{mode} gave one side {count} whites out of {repeats} meetings"
            );
        }
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
            assert!(
                (white - black).abs() <= 1,
                "{mode} left a player on {white} whites and {black} blacks"
            );
        }

        // Every game knows its round, and nobody is booked twice in one.
        let mut per_round: HashMap<i32, Vec<&db_lib::models::Game>> = HashMap::new();
        for game in &games {
            let round = game.round.expect("every tournament game records its round");
            per_round.entry(round).or_default().push(game);
        }
        assert_eq!(per_round.len(), expected_rounds(players, repeats));
        for (round, round_games) in &per_round {
            let mut booked = HashSet::new();
            for game in round_games {
                assert!(
                    booked.insert(game.white_id),
                    "a player is booked twice in round {round}"
                );
                assert!(
                    booked.insert(game.black_id),
                    "a player is booked twice in round {round}"
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn double_round_robin_ends_in_seed_order_with_balanced_colours() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
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
        vec![vec![0], vec![1], vec![2], vec![3]],
    );
    // Each player meets the other three twice.
    assert_eq!(
        score(&standings, fixture.players[0].id, Tiebreaker::RawPoints),
        6.0
    );
    assert_eq!(
        score(&standings, fixture.players[3].id, Tiebreaker::RawPoints),
        0.0
    );
    assert_points_conserved(&standings, 12);
}

/// The strongest available check on the scoring: play a full double round
/// robin with a real mix of wins, draws and losses, tally the points
/// independently from the game rows, and require the engine to agree — rather
/// than asserting against a ladder the test itself dictated.
#[tokio::test(flavor = "multi_thread")]
async fn double_round_robin_standings_match_an_independent_tally_of_every_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
        TournamentOpts {
            player_count: 6,
            tiebreakers: vec![
                Tiebreaker::RawPoints,
                Tiebreaker::SonnebornBerger,
                Tiebreaker::Buchholz,
                Tiebreaker::Wins,
            ],
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let scheduled = start(&mut fixture, &mut conn).await;

    // Six players meeting twice is 30 games over 10 rounds.
    assert_eq!(scheduled.len(), 30);
    assert_eq!(expected_rounds(6, 2), 10);

    // Every pair meets exactly twice, once with each colour — the return leg
    // swaps, which is the whole point of a double round robin.
    let (met, whites) = meetings(&scheduled);
    assert_eq!(met.len(), 15, "every one of the fifteen pairs meets");
    assert!(met.values().all(|count| *count == 2));
    assert!(
        whites.values().all(|count| *count == 1),
        "each pair plays one game with each player as white"
    );

    play_unfinished(
        &fixture,
        |game| {
            scripted_result(
                game.round.expect("round is recorded"),
                fixture.seed_of(game.white_id),
                fixture.seed_of(game.black_id),
            )
        },
        &mut conn,
    )
    .await;
    assert!(matches!(
        fixture.tournament.progress(&mut conn).await.unwrap(),
        ProgressOutcome::ReadyToFinish
    ));

    // Tally the same games by hand, straight off the rows.
    let played = games_of(&fixture, &mut conn).await;
    assert_eq!(played.len(), 30);
    let mut tally: HashMap<uuid::Uuid, f32> = HashMap::new();
    let mut wins: HashMap<uuid::Uuid, f32> = HashMap::new();
    for game in &played {
        assert!(game.finished, "every game was adjudicated");
        match TournamentGameResult::from_str(&game.tournament_game_result)
            .expect("a stored result parses")
        {
            TournamentGameResult::Draw => {
                *tally.entry(game.white_id).or_default() += 0.5;
                *tally.entry(game.black_id).or_default() += 0.5;
            }
            TournamentGameResult::Winner(Color::White) => {
                *tally.entry(game.white_id).or_default() += 1.0;
                *wins.entry(game.white_id).or_default() += 1.0;
            }
            TournamentGameResult::Winner(Color::Black) => {
                *tally.entry(game.black_id).or_default() += 1.0;
                *wins.entry(game.black_id).or_default() += 1.0;
            }
            other => panic!("unexpected scripted result {other}"),
        }
    }

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 6);
    assert_points_conserved(&standings, 30);

    for player in &fixture.players {
        let expected = tally.get(&player.id).copied().unwrap_or(0.0);
        assert!(
            (score(&standings, player.id, Tiebreaker::RawPoints) - expected).abs() < 1e-4,
            "{} scored {expected} by hand",
            fixture.name_of(player.id)
        );
        assert_eq!(
            score(&standings, player.id, Tiebreaker::Wins),
            wins.get(&player.id).copied().unwrap_or(0.0),
        );
        assert_eq!(
            standings
                .players()
                .find(|standing| standing.player == player.id)
                .expect("player is ranked")
                .games_played,
            10,
            "each player meets the other five twice"
        );
    }

    // The mix must have actually produced draws and a real spread, or the
    // cross-check above would be trivially satisfied by a dominance ladder.
    assert!(
        played
            .iter()
            .any(|game| game.tournament_game_result == TournamentGameResult::Draw.to_string()),
        "the script should produce at least one draw"
    );
    assert!(
        standings.groups.len() > 1,
        "the field should not end up entirely level"
    );

    // Sonneborn-Berger weights wins by the strength of who you beat, so it can
    // only be meaningful if it is actually populated and finite.
    for player in &fixture.players {
        for tiebreaker in [Tiebreaker::SonnebornBerger, Tiebreaker::Buchholz] {
            let value = score(&standings, player.id, tiebreaker);
            assert!(
                value.is_finite() && value >= 0.0,
                "{tiebreaker} for {} was {value}",
                fixture.name_of(player.id)
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn odd_field_sits_each_player_out_once_per_pass_and_a_bye_is_worth_nothing() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DoubleRoundRobin,
        TournamentOpts {
            player_count: 5,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    // Byes are recorded so the UI can say who sat out, but a round-robin bye
    // is a rest rather than a result — unlike Swiss, it is worth no points.
    let byes = byes_of(&fixture, &mut conn).await;
    assert_eq!(byes.len(), 10, "one player sits out each of the ten rounds");
    let mut byes_per_round: HashMap<i32, usize> = HashMap::new();
    let mut byes_per_player: HashMap<uuid::Uuid, usize> = HashMap::new();
    for bye in &byes {
        *byes_per_round.entry(bye.round).or_default() += 1;
        *byes_per_player.entry(bye.user_id).or_default() += 1;
    }
    assert!(byes_per_round.values().all(|count| *count == 1));
    assert!(
        byes_per_player.values().all(|count| *count == 2),
        "every player rests once per pass"
    );

    let games = games_of(&fixture, &mut conn).await;
    for player in &fixture.players {
        let played = games
            .iter()
            .filter(|game| game.white_id == player.id || game.black_id == player.id)
            .count();
        assert_eq!(played, 8, "each player meets the other four twice");
    }

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 5);
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2], vec![3], vec![4]],
    );
    assert_eq!(
        score(&standings, fixture.players[0].id, Tiebreaker::RawPoints),
        8.0
    );
    assert_points_conserved(&standings, 20);
}

/// The interesting case for a format whose whole schedule exists up front:
/// hive lets players play their games whenever they like, so standings have to
/// be right even when late rounds finish before early ones.
#[tokio::test(flavor = "multi_thread")]
async fn standings_are_correct_when_games_finish_out_of_order() {
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

    let all = games_of(&fixture, &mut conn).await;
    let mut rounds: Vec<i32> = all.iter().filter_map(|game| game.round).collect();
    rounds.sort_unstable();
    rounds.dedup();
    assert_eq!(rounds, vec![1, 2, 3]);

    // Play the last round first, and only that round.
    let mut expected: HashMap<uuid::Uuid, f32> = HashMap::new();
    for game in all.iter().filter(|game| game.round == Some(3)) {
        let winner = if fixture.seed_of(game.white_id) < fixture.seed_of(game.black_id) {
            (game.white_id, Color::White)
        } else {
            (game.black_id, Color::Black)
        };
        game.adjudicate_tournament_result(
            &fixture.organizer.id,
            &TournamentGameResult::Winner(winner.1),
            &mut conn,
        )
        .await
        .expect("adjudicate a round three game");
        *expected.entry(winner.0).or_default() += 1.0;
    }

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    for player in &fixture.players {
        assert_eq!(
            score(&standings, player.id, Tiebreaker::RawPoints),
            expected.get(&player.id).copied().unwrap_or(0.0),
            "only the finished round three counts so far"
        );
    }
    assert!(
        matches!(
            fixture.tournament.progress(&mut conn).await.unwrap(),
            ProgressOutcome::Waiting
        ),
        "rounds one and two are still outstanding"
    );

    // Now everything else, and the final table must match the in-order run.
    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2], vec![3]],
        "finishing out of order must not change the final standings"
    );
    assert_points_conserved(&standings, 6);
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_waits_until_every_game_is_played() {
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

    assert!(matches!(
        fixture.tournament.progress(&mut conn).await.unwrap(),
        ProgressOutcome::Waiting
    ));

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, result, &mut conn).await;

    assert!(matches!(
        fixture.tournament.progress(&mut conn).await.unwrap(),
        ProgressOutcome::ReadyToFinish
    ));
    common::tournament::finish(&fixture, &mut conn).await;
}

/// Two identical tournaments differing only in their configured tiebreaker
/// must break the same tie differently.
#[tokio::test(flavor = "multi_thread")]
async fn the_configured_tiebreaker_decides_a_tie() {
    let mut orderings = Vec::new();
    for tiebreaker in [Tiebreaker::WinsAsBlack, Tiebreaker::SonnebornBerger] {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            TournamentMode::DoubleRoundRobin,
            TournamentOpts {
                player_count: 3,
                tiebreakers: vec![Tiebreaker::RawPoints, tiebreaker],
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        start(&mut fixture, &mut conn).await;

        // Everyone beats the next player round-robin style, so all three end
        // level on two points and only the tiebreaker separates them.
        let seeds_by_id: HashMap<uuid::Uuid, usize> = fixture
            .players
            .iter()
            .enumerate()
            .map(|(seed, user)| (user.id, seed))
            .collect();
        play_unfinished(
            &fixture,
            |game| {
                let white = seeds_by_id[&game.white_id];
                let black = seeds_by_id[&game.black_id];
                let white_wins = (white + 1) % 3 == black;
                TournamentGameResult::Winner(if white_wins {
                    Color::White
                } else {
                    Color::Black
                })
            },
            &mut conn,
        )
        .await;

        let standings = standings_of(&fixture, &mut conn).await;
        assert_well_formed(&standings, 3);
        for player in &fixture.players {
            assert_eq!(
                score(&standings, player.id, Tiebreaker::RawPoints),
                2.0,
                "the cycle leaves all three level on points"
            );
        }
        assert!(
            standings.tiebreakers.contains(&tiebreaker),
            "the configured tiebreaker must be applied"
        );
        orderings.push(fixture.seed_groups(&standings));
    }

    assert_eq!(orderings.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn seeds_follow_rating_and_match_the_engines_player_ids() {
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

    let stored = seeds(&fixture, &mut conn).await;
    assert_eq!(stored.len(), 4);
    for (index, (user, seed)) in stored.iter().enumerate() {
        assert_eq!(*seed, index as i32, "seeds must be contiguous from zero");
        assert_eq!(
            *user, fixture.players[index].id,
            "the highest rated player takes seed zero"
        );
    }

    // Round one of the circle method pairs the top seed with the bottom one.
    let games = games_of(&fixture, &mut conn).await;
    let first: Vec<&db_lib::models::Game> =
        games.iter().filter(|game| game.round == Some(1)).collect();
    assert_eq!(first.len(), 2);
    assert!(first.iter().any(|game| {
        unordered(game.white_id, game.black_id)
            == unordered(fixture.players[0].id, fixture.players[3].id)
    }));
}
