mod common;

use common::tournament::{
    assert_well_formed,
    create_tournament,
    games_of,
    lower_seed_wins,
    play_unfinished,
    run_to_completion,
    score,
    standings_of,
    start,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome};
use hive_lib::Color;
use shared_types::{
    Conclusion,
    PointSystemDetails,
    ScoringMode,
    Tiebreaker,
    TournamentGameResult,
    TournamentMode,
};
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread")]
async fn a_tournament_can_set_its_own_point_values() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    // Football scoring: three for a win, one for a draw.
    let mut fixture = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            points: PointSystemDetails {
                win: Some(3.0),
                draw: Some(1.0),
                loss: Some(0.0),
                ..Default::default()
            },
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
    // The top seed beats all three others, at three points each.
    assert_eq!(
        score(&standings, fixture.players[0].id, Tiebreaker::RawPoints),
        9.0,
        "a win is worth what the tournament says it is"
    );
    assert_eq!(
        score(&standings, fixture.players[3].id, Tiebreaker::RawPoints),
        0.0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forfeit_can_be_scored_differently_from_a_loss() {
    for (forfeit_loss, expected) in [(0.0, 0.0), (0.5, 0.5)] {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            TournamentMode::DutchSwiss,
            TournamentOpts {
                player_count: 4,
                rounds: 1,
                points: PointSystemDetails {
                    forfeit_loss: Some(forfeit_loss),
                    ..Default::default()
                },
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
                expected,
                "a no-show should be worth the configured {forfeit_loss}"
            );
        }
    }
}

/// The engine doubles every value so a draw stays a whole number, which makes
/// half a point the finest step a tournament can ask for.
#[test]
fn point_values_must_be_whole_half_points() {
    let with_win = |win: f64| PointSystemDetails {
        win: Some(win),
        ..Default::default()
    };

    assert!(with_win(1.0).is_valid());
    assert!(with_win(1.5).is_valid(), "halves are representable");
    assert!(with_win(3.0).is_valid(), "so is football scoring");
    assert!(!with_win(0.3).is_valid(), "a third of a point is not");
    assert!(!with_win(-1.0).is_valid(), "nor is a negative score");
    assert!(
        PointSystemDetails::default().is_valid(),
        "setting nothing at all is always valid — the mode decides"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tournament_with_unrepresentable_points_is_rejected() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    // Kept honest by creating a real tournament either side of the bad one.
    let mut fine = create_tournament(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            points: PointSystemDetails {
                win: Some(2.5),
                ..Default::default()
            },
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fine, &mut conn).await;

    let details = common::tournament::details_for(
        TournamentMode::SingleRoundRobin,
        TournamentOpts {
            player_count: 4,
            points: PointSystemDetails {
                win: Some(0.3),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(
        db_lib::models::NewTournament::new(details).is_err(),
        "a third of a point cannot be stored, so it is refused up front"
    );
}

/// The World Championship case: Double-Swiss scored by games should let the
/// tiebreaks see game points too, not just the match result.
#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_scoring_mode_selects_the_tiebreak_unit() {
    let mut progressive = Vec::new();
    for scoring in [ScoringMode::Match, ScoringMode::Game] {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            TournamentMode::DoubleSwiss,
            TournamentOpts {
                player_count: 4,
                rounds: 1,
                scoring,
                tiebreakers: vec![Tiebreaker::RawPoints, Tiebreaker::ProgressiveScore],
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        let games = start(&mut fixture, &mut conn).await;

        // One pairing is swept 2-0, the other won 1½-½. Both are match wins.
        let first_pair = (games[0].white_id, games[0].black_id);
        let mut sweeper = None;
        for game in &games {
            let same_pair = (game.white_id == first_pair.0 && game.black_id == first_pair.1)
                || (game.white_id == first_pair.1 && game.black_id == first_pair.0);
            let result = if same_pair {
                sweeper = Some(first_pair.0);
                if game.white_id == first_pair.0 {
                    TournamentGameResult::Winner(Color::White)
                } else {
                    TournamentGameResult::Winner(Color::Black)
                }
            } else {
                // The other pairing: one win and one draw.
                let lower = game.white_id.min(game.black_id);
                if game.white_id == lower {
                    TournamentGameResult::Winner(Color::White)
                } else {
                    TournamentGameResult::Draw
                }
            };
            game.adjudicate_tournament_result(&fixture.organizer.id, &result, &mut conn)
                .await
                .expect("adjudicate");
        }

        let standings = standings_of(&fixture, &mut conn).await;
        assert_well_formed(&standings, 4);
        progressive.push(score(
            &standings,
            sweeper.expect("a sweeper"),
            Tiebreaker::ProgressiveScore,
        ));
    }

    assert!(
        progressive[1] >= progressive[0],
        "scoring by games should never value a sweep less than scoring by matches"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_from_a_round_robin_forfeits_the_rest_and_can_be_undone() {
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

    let leaving = fixture.players[3].id;
    let forfeited = fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("a player may withdraw themselves");
    assert_eq!(forfeited, 3, "their three remaining games are forfeited");

    let games = games_of(&fixture, &mut conn).await;
    let theirs: Vec<&db_lib::models::Game> = games
        .iter()
        .filter(|game| game.white_id == leaving || game.black_id == leaving)
        .collect();
    assert!(theirs.iter().all(|game| game.finished));
    assert!(
        theirs.iter().all(|game| matches!(
            Conclusion::from_str(&game.conclusion),
            Ok(Conclusion::Withdrawal)
        )),
        "marked as a withdrawal so reinstating can undo exactly these"
    );

    // Their opponents got the points.
    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(score(&standings, leaving, Tiebreaker::RawPoints), 0.0);

    // An organizer changes their mind.
    let restored = fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("an organizer may reinstate");
    assert_eq!(restored, 3, "the same three games are playable again");

    let games = games_of(&fixture, &mut conn).await;
    let theirs: Vec<&db_lib::models::Game> = games
        .iter()
        .filter(|game| game.white_id == leaving || game.black_id == leaving)
        .collect();
    assert!(
        theirs.iter().all(|game| !game.finished),
        "reinstating puts the games back"
    );

    let result = lower_seed_wins(&fixture);
    run_to_completion(&fixture, result, &mut conn).await;
    let standings = standings_of(&fixture, &mut conn).await;
    assert_eq!(
        fixture.seed_groups(&standings),
        vec![vec![0], vec![1], vec![2], vec![3]],
        "the reinstated player finishes normally"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn reinstating_leaves_an_organizers_own_adjudications_alone() {
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

    // The organizer settles one game by hand first.
    let leaving = fixture.players[3].id;
    let games = games_of(&fixture, &mut conn).await;
    let adjudicated = games
        .iter()
        .find(|game| game.white_id == leaving || game.black_id == leaving)
        .expect("they have games");
    adjudicated
        .adjudicate_tournament_result(
            &fixture.organizer.id,
            &TournamentGameResult::Winner(Color::White),
            &mut conn,
        )
        .await
        .expect("organizer ruling");

    fixture
        .tournament
        .withdraw_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("organizer withdraws them");
    fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("and puts them back");

    let games = games_of(&fixture, &mut conn).await;
    let still_settled = games
        .iter()
        .find(|game| game.id == adjudicated.id)
        .expect("the adjudicated game");
    assert!(
        still_settled.finished,
        "an organizer's own ruling survives a withdraw/reinstate cycle"
    );
    assert!(matches!(
        Conclusion::from_str(&still_settled.conclusion),
        Ok(Conclusion::Committee)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_withdrawn_swiss_player_is_not_paired_again() {
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
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    let leaving = fixture.players[0].id;
    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");

    let ProgressOutcome::Advanced(next) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the survivors should still be paired");
    };
    for game in &next {
        assert!(
            game.white_id != leaving && game.black_id != leaving,
            "a withdrawn player is not paired again"
        );
    }
}

/// `withdraw_player` is the entry point everything outside `db` will use, so
/// it has to work for every format — not just the ones with a bespoke path.
#[tokio::test(flavor = "multi_thread")]
async fn the_generic_withdrawal_works_in_every_mode() {
    for mode in [
        TournamentMode::SingleRoundRobin,
        TournamentMode::DutchSwiss,
        TournamentMode::DoubleSwiss,
        TournamentMode::SingleElimination,
        TournamentMode::DoubleElimination,
        TournamentMode::Arena,
    ] {
        let db = common::db::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get connection");

        let mut fixture = create_tournament(
            mode,
            TournamentOpts {
                player_count: 4,
                rounds: if mode.is_swiss() { 2 } else { 0 },
                arena_duration_seconds: mode.is_arena().then_some(3600),
                seats: mode.is_arena().then_some(12),
                ..Default::default()
            },
            &mut conn,
        )
        .await;
        start(&mut fixture, &mut conn).await;
        if mode.is_arena() {
            // An arena has nothing paired until its first tick.
            fixture.tournament.progress(&mut conn).await.expect("tick");
        }

        let leaving = fixture.players[0].id;
        fixture
            .tournament
            .withdraw_player(&leaving, &leaving, &mut conn)
            .await
            .unwrap_or_else(|error| panic!("{mode} should accept a withdrawal: {error:?}"));

        // Whatever the format, they must not turn up in anything new.
        let outcome = fixture
            .tournament
            .progress(&mut conn)
            .await
            .unwrap_or_else(|error| panic!("{mode} should still progress: {error:?}"));
        if let ProgressOutcome::Advanced(games) | ProgressOutcome::Replays(games) = outcome {
            for game in &games {
                assert!(
                    game.white_id != leaving && game.black_id != leaving,
                    "{mode} paired a player who had withdrawn"
                );
            }
        }

        let standings = standings_of(&fixture, &mut conn).await;
        assert_well_formed(&standings, 4);
    }
}

/// A game already under way is a real game with real ratings: withdrawal
/// resigns it rather than wiping it, and reinstating cannot bring it back.
#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_resigns_a_game_in_progress_instead_of_resetting_it() {
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

    // Put moves on one of their games.
    let leaving = fixture.players[3].id;
    let games = games_of(&fixture, &mut conn).await;
    let underway = games
        .iter()
        .find(|game| game.white_id == leaving || game.black_id == leaving)
        .expect("they have games");
    {
        use db_lib::schema::games as games_table;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        diesel::update(games_table::table.find(underway.id))
            .set((
                games_table::turn.eq(2),
                games_table::history.eq("wS1 .;bG1 -wS1;"),
                games_table::game_status.eq("InProgress"),
                // A real in-progress game always has one; without it the clock
                // check panics on a `todo!()` in `timed_out_color`.
                games_table::last_interaction.eq(Some(chrono::Utc::now())),
            ))
            .execute(&mut conn)
            .await
            .expect("put the game under way");
    }

    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");

    let games = games_of(&fixture, &mut conn).await;
    let played = games
        .iter()
        .find(|game| game.id == underway.id)
        .expect("the started game");
    assert!(played.finished);
    assert!(
        matches!(
            Conclusion::from_str(&played.conclusion),
            Ok(Conclusion::Resigned)
        ),
        "a started game is resigned, not marked as a withdrawal forfeit"
    );
    assert_eq!(played.turn, 2, "its moves are left intact");

    // Reinstating restores only the never-started games.
    fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("reinstate");
    let games = games_of(&fixture, &mut conn).await;
    let played = games
        .iter()
        .find(|game| game.id == underway.id)
        .expect("the started game");
    assert!(
        played.finished,
        "a resigned game stays resigned — its rating change already happened"
    );
    assert!(
        games
            .iter()
            .filter(|game| game.id != underway.id)
            .filter(|game| game.white_id == leaving || game.black_id == leaving)
            .all(|game| !game.finished),
        "the untouched games are playable again"
    );
}

/// Withdrawal and reinstatement have to be symmetric in an arena too:
/// withdrawing forfeits the player's unstarted game, so reinstating has to give
/// it back, or they are left out of a game they never agreed to leave.
#[tokio::test(flavor = "multi_thread")]
async fn reinstating_in_an_arena_gives_back_the_forfeited_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::Arena,
        TournamentOpts {
            player_count: 2,
            arena_duration_seconds: Some(3600),
            seats: Some(10),
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;
    fixture.tournament.progress(&mut conn).await.expect("tick");

    let leaving = fixture.players[0].id;
    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");
    assert!(
        games_of(&fixture, &mut conn)
            .await
            .iter()
            .all(|game| game.finished),
        "their unstarted game is forfeited so the arena can still finish"
    );

    let restored = fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("reinstate");
    assert_eq!(restored, 1, "the forfeited game is given back");
    assert!(
        games_of(&fixture, &mut conn)
            .await
            .iter()
            .all(|game| !game.finished),
        "and is playable again"
    );

    // The arena is still coherent afterwards.
    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_player_or_an_organizer_may_withdraw_them() {
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

    let victim = fixture.players[0].id;
    let meddler = fixture.players[1].id;
    assert!(
        fixture
            .tournament
            .withdraw_player(&victim, &meddler, &mut conn)
            .await
            .is_err(),
        "one player cannot withdraw another"
    );

    fixture
        .tournament
        .withdraw_player(&victim, &fixture.organizer.id, &mut conn)
        .await
        .expect("an organizer can");
    assert!(
        fixture
            .tournament
            .withdraw_player(&victim, &victim, &mut conn)
            .await
            .is_err(),
        "withdrawing twice is refused"
    );
    assert!(
        fixture
            .tournament
            .reinstate_player(&victim, &meddler, &mut conn)
            .await
            .is_err(),
        "reinstating is organizers only"
    );
}

/// `reset_adjudicated_games` undoes an organizer's rulings. A withdrawal
/// forfeit is adjudicated too, but it is not a ruling — clearing it leaves a
/// game nobody will ever play and destroys the conclusion that reinstating
/// finds it by, so there is no way back.
#[tokio::test(flavor = "multi_thread")]
async fn resetting_adjudicated_games_leaves_withdrawal_forfeits_alone() {
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

    let leaving = fixture.players[3].id;
    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");

    fixture
        .tournament
        .reset_adjudicated_games(&fixture.organizer.id, &mut conn)
        .await
        .expect("reset the organizer's rulings");

    let forfeited: Vec<_> = games_of(&fixture, &mut conn)
        .await
        .into_iter()
        .filter(|game| {
            matches!(
                Conclusion::from_str(&game.conclusion),
                Ok(Conclusion::Withdrawal)
            )
        })
        .collect();
    assert!(
        !forfeited.is_empty(),
        "the withdrawal forfeits must survive a reset of adjudicated games"
    );

    // And the repair path still works, which it would not if the conclusion had
    // been overwritten.
    fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("reinstate");
}

/// Reinstating must not re-open a game in a round the tournament has already
/// moved past: replay would never resolve that round, and the next
/// `begin_round` would refuse because the round it holds never closed.
#[tokio::test(flavor = "multi_thread")]
async fn reinstating_does_not_reopen_a_round_already_left_behind() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 3,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;
    assert!(matches!(
        fixture
            .tournament
            .progress(&mut conn)
            .await
            .expect("round two"),
        ProgressOutcome::Advanced(_)
    ));

    // Out during round two, then straight back in.
    let leaving = fixture.players[3].id;
    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");
    fixture
        .tournament
        .reinstate_player(&leaving, &fixture.organizer.id, &mut conn)
        .await
        .expect("reinstate");

    // Round one must still be settled, so the tournament can still be read and
    // still advances.
    let round_one_open = games_of(&fixture, &mut conn)
        .await
        .into_iter()
        .any(|game| game.round == Some(1) && !game.finished);
    assert!(
        !round_one_open,
        "reinstating must not re-open a game in a round that has already closed"
    );
    fixture
        .tournament
        .standings(&mut conn)
        .await
        .expect("standings still compute after a reinstatement");
}

/// `points_zero_point_bye` was configurable but unreachable: nothing could
/// produce a bye that was not the pairing-allocated kind, so the setting could
/// not move any score.
#[tokio::test(flavor = "multi_thread")]
async fn a_granted_zero_point_bye_sits_a_player_out_and_pays_its_own_setting() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 5,
            rounds: 2,
            points: PointSystemDetails {
                // Deliberately not zero, so reaching it is observable, and
                // distinct from the full-point pairing-allocated bye.
                zero_point_bye: Some(0.5),
                ..Default::default()
            },
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    let result = lower_seed_wins(&fixture);
    play_unfinished(&fixture, &result, &mut conn).await;

    let sitting_out = fixture.players[0].id;
    fixture
        .tournament
        .grant_zero_point_bye(&sitting_out, &fixture.organizer.id, &mut conn)
        .await
        .expect("organizer grants a bye for the round about to be paired");

    let before = standings_of(&fixture, &mut conn).await;
    let earned = score(&before, sitting_out, Tiebreaker::RawPoints);

    let ProgressOutcome::Advanced(games) = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("pair round two")
    else {
        panic!("round two should be paired");
    };
    assert!(
        games
            .iter()
            .all(|game| game.white_id != sitting_out && game.black_id != sitting_out),
        "a player sitting out must not be paired into the round"
    );

    let after = standings_of(&fixture, &mut conn).await;
    assert_eq!(
        score(&after, sitting_out, Tiebreaker::RawPoints),
        earned + 0.5,
        "the bye pays points_zero_point_bye, not the full-point pairing bye"
    );
}
