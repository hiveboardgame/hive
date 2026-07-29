mod common;

use common::tournament::{
    assert_well_formed,
    create_tournament,
    create_user,
    games_of,
    score,
    standings_of,
    start,
    TournamentOpts,
};
use db_lib::{get_conn, models::ProgressOutcome, DbConn};
use hive_lib::Color;
use shared_types::{Tiebreaker, TournamentGameResult, TournamentMode};
use std::collections::HashSet;

const HOUR: i32 = 3600;

/// An arena with room to spare, because people turn up after it starts.
fn arena(players: usize) -> TournamentOpts {
    TournamentOpts {
        player_count: players,
        arena_duration_seconds: Some(HOUR),
        tiebreakers: vec![Tiebreaker::RawPoints],
        seats: Some(players as i32 + 8),
        ..Default::default()
    }
}

/// An arena with no spare seats, for testing the capacity guard.
fn full_arena(players: usize) -> TournamentOpts {
    TournamentOpts {
        seats: Some(players as i32),
        ..arena(players)
    }
}

/// Ticks the arena and asserts `player` is in none of the games that tick
/// created.
///
/// Deliberately not asserted against `ProgressOutcome::Advanced`: a tick that
/// pairs nobody returns `Waiting`, and matching only the `Advanced` case would
/// make the guarantee vacuously true exactly when it is hardest to hold.
/// `Tournament::games` is unordered, so the new rows are found by id.
async fn progress_and_assert_unpaired(
    fixture: &common::tournament::Fixture,
    player: uuid::Uuid,
    message: &str,
    conn: &mut DbConn<'_>,
) {
    let before: HashSet<uuid::Uuid> = games_of(fixture, conn).await.iter().map(|g| g.id).collect();
    fixture.tournament.progress(conn).await.unwrap();
    for game in games_of(fixture, conn).await {
        if before.contains(&game.id) {
            continue;
        }
        assert!(
            game.white_id != player && game.black_id != player,
            "{message}"
        );
    }
}

/// Arena pairing depends on when each game *ended*, so a result has to be
/// stamped at a chosen instant rather than "now" — otherwise every game in a
/// test would finish in the same millisecond.
async fn finish_at(
    game: &db_lib::models::Game,
    result: TournamentGameResult,
    seconds_in: i64,
    started_at: chrono::DateTime<chrono::Utc>,
    conn: &mut DbConn<'_>,
) {
    use db_lib::schema::games;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    game.assign_tournament_result(&result, conn)
        .await
        .expect("record arena result");
    diesel::update(games::table.find(game.id))
        .set(games::updated_at.eq(started_at + chrono::Duration::seconds(seconds_in)))
        .execute(conn)
        .await
        .expect("stamp the finish time");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_arena_starts_empty_and_pairs_on_its_first_tick() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    let opening = start(&mut fixture, &mut conn).await;

    // Unlike every other format, an arena has nothing paired at start: the
    // clock has to exist before anyone can be paired against it.
    assert!(
        opening.is_empty(),
        "an arena opens with no games, it pairs on the first tick"
    );

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("first arena tick");
    let ProgressOutcome::Advanced(paired) = outcome else {
        panic!("the first tick should pair the waiting pool, got {outcome:?}");
    };
    assert_eq!(paired.len(), 2, "four waiting players make two games");

    // Everyone is paired exactly once, and each game carries the arena's own
    // id rather than a round number.
    let mut seen = HashSet::new();
    for game in &paired {
        assert!(game.round.is_none(), "an arena has no rounds");
        assert!(
            game.arena_game_id.is_some(),
            "an arena game records the id the engine knows it by"
        );
        assert!(seen.insert(game.white_id));
        assert!(seen.insert(game.black_id));
    }
    assert_eq!(seen.len(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_player_is_repaired_as_soon_as_their_own_game_ends() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };
    assert_eq!(first.len(), 2);

    // Only one of the two games finishes. Those two players are free again
    // with no round to wait for — but they have just played each other, and
    // the rematch cooldown means they cannot immediately go again, so nothing
    // can be paired until somebody else is free.
    finish_at(
        &first[0],
        TournamentGameResult::Winner(Color::White),
        60,
        started_at,
        &mut conn,
    )
    .await;

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("second arena tick");
    assert!(
        matches!(outcome, ProgressOutcome::Waiting),
        "the only two free players just played each other, got {outcome:?}"
    );

    // Once the other game ends too, all four are free and everyone gets a new
    // opponent rather than a rematch.
    finish_at(
        &first[1],
        TournamentGameResult::Winner(Color::White),
        70,
        started_at,
        &mut conn,
    )
    .await;

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("third arena tick");
    let ProgressOutcome::Advanced(second) = outcome else {
        panic!("four free players should be paired again, got {outcome:?}");
    };
    assert_eq!(second.len(), 2, "all four players are free");

    let already_met: HashSet<(uuid::Uuid, uuid::Uuid)> = first
        .iter()
        .map(|game| common::tournament::unordered(game.white_id, game.black_id))
        .collect();
    for game in &second {
        assert!(
            !already_met.contains(&common::tournament::unordered(game.white_id, game.black_id)),
            "nobody should be rematched with the opponent they just played"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_late_joiner_is_seeded_on_arrival_and_gets_paired() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(2), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair its two starters");
    };
    assert_eq!(first.len(), 1);

    // Two more players turn up while the clock runs — the thing that makes an
    // arena an arena.
    let late_one = create_user("late_one", 1900.0, &mut conn).await;
    let late_two = create_user("late_two", 1890.0, &mut conn).await;
    for user in [&late_one, &late_two] {
        let entrant = fixture
            .tournament
            .join_arena(&user.id, &mut conn)
            .await
            .expect("join the running arena");
        assert!(
            entrant.seed.is_some(),
            "an arena entrant is seeded on arrival, not at start"
        );
        assert!(entrant.joined_at.is_some());
    }

    let ProgressOutcome::Advanced(second) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the two late joiners should be paired with each other");
    };
    assert_eq!(second.len(), 1);
    let paired: HashSet<uuid::Uuid> = [second[0].white_id, second[0].black_id]
        .into_iter()
        .collect();
    assert_eq!(
        paired,
        [late_one.id, late_two.id]
            .into_iter()
            .collect::<HashSet<_>>(),
        "the original two are still playing, so the newcomers meet"
    );

    finish_at(
        &first[0],
        TournamentGameResult::Winner(Color::White),
        30,
        started_at,
        &mut conn,
    )
    .await;
    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        standings.groups.iter().map(Vec::len).sum::<usize>(),
        4,
        "a late joiner is ranked alongside everyone else"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn arena_standings_score_wins_and_rank_by_points_then_fewest_games() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };

    // Both openers are decisive; white takes each.
    let mut winners = Vec::new();
    let mut losers = Vec::new();
    for (index, game) in first.iter().enumerate() {
        finish_at(
            game,
            TournamentGameResult::Winner(Color::White),
            60 + index as i64,
            started_at,
            &mut conn,
        )
        .await;
        winners.push(game.white_id);
        losers.push(game.black_id);
    }

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);

    for winner in &winners {
        assert_eq!(
            score(&standings, *winner, Tiebreaker::RawPoints),
            1.0,
            "a win is worth a point"
        );
        assert_eq!(score(&standings, *winner, Tiebreaker::Wins), 1.0);
        assert_eq!(score(&standings, *winner, Tiebreaker::GamesPlayed), 1.0);
    }
    for loser in &losers {
        assert_eq!(score(&standings, *loser, Tiebreaker::RawPoints), 0.0);
        assert_eq!(score(&standings, *loser, Tiebreaker::Losses), 1.0);
    }

    // Winners above losers, and the arena's own key order is reported.
    let top: HashSet<uuid::Uuid> = standings.groups[0].iter().map(|s| s.player).collect();
    assert_eq!(top, winners.iter().copied().collect::<HashSet<_>>());
    assert_eq!(
        &standings.tiebreakers[..3],
        &[
            Tiebreaker::RawPoints,
            Tiebreaker::Wins,
            Tiebreaker::GamesPlayed
        ],
        "an arena ranks on points, then wins, then fewest games"
    );
}

/// Two wins in a row start doubling on lichess, and that is the main reason an
/// arena's table does not look like a plain win count.
#[tokio::test(flavor = "multi_thread")]
async fn a_winning_streak_is_worth_more_than_the_same_wins_apart() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let mut streaker: Option<uuid::Uuid> = None;
    let mut elapsed = 60i64;
    for round in 0..3 {
        let outcome = fixture.tournament.progress(&mut conn).await.unwrap();
        // Four players, all idle, so every tick pairs two games and the
        // streaker plays in each. Breaking out here instead would let the
        // streak never form and the assertion below pass on nothing.
        let ProgressOutcome::Advanced(games) = outcome else {
            panic!("tick {round} paired nobody, so no streak can build: {outcome:?}");
        };
        for game in &games {
            // Whoever won the opener keeps winning, so their streak builds.
            let winner_is_white = match streaker {
                None => true,
                Some(player) => game.white_id == player,
            };
            let result = if winner_is_white {
                TournamentGameResult::Winner(Color::White)
            } else {
                TournamentGameResult::Winner(Color::Black)
            };
            if streaker.is_none() {
                streaker = Some(game.white_id);
            }
            finish_at(game, result, elapsed, started_at, &mut conn).await;
            elapsed += 30;
        }
    }

    let streaker = streaker.expect("someone won the first game");
    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);

    let wins = score(&standings, streaker, Tiebreaker::Wins);
    let points = score(&standings, streaker, Tiebreaker::RawPoints);
    let best_streak = score(&standings, streaker, Tiebreaker::BestStreak);
    assert_eq!(wins, 3.0, "the streaker won their game in all three ticks");
    assert_eq!(best_streak, 3.0, "and won them back to back");
    assert!(
        points > wins,
        "with a streak of {best_streak}, {wins} wins should be worth more than {points} points"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_arena_is_over_once_its_clock_runs_out() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    // A one-second arena: by the time the first tick happens it has expired.
    let mut fixture = create_tournament(
        TournamentMode::Arena,
        TournamentOpts {
            player_count: 2,
            arena_duration_seconds: Some(1),
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut fixture, &mut conn).await;

    use db_lib::schema::tournaments;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    diesel::update(tournaments::table.find(fixture.tournament.id))
        .set(tournaments::started_at.eq(chrono::Utc::now() - chrono::Duration::seconds(120)))
        .execute(&mut conn)
        .await
        .expect("age the arena past its duration");

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("tick an expired arena");
    assert!(
        matches!(outcome, ProgressOutcome::ReadyToFinish),
        "an expired arena with no games in flight is over, got {outcome:?}"
    );
    assert!(
        games_of(&fixture, &mut conn).await.is_empty(),
        "an expired arena pairs nobody"
    );
}

/// Berserk trades clock for points: half the base and the whole increment go,
/// and a berserked win scores more than a plain one.
#[tokio::test(flavor = "multi_thread")]
async fn berserking_halves_the_clock_and_pays_a_bonus_on_a_win() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };

    let before = first[0].white_time_left.expect("a real time control");
    let berserked = first[0]
        .berserk(Color::White, &mut conn)
        .await
        .expect("declare berserk");

    assert!(berserked.white_berserked);
    assert!(!berserked.black_berserked, "only one side declared");
    assert_eq!(
        berserked.white_time_left,
        Some(before / 2),
        "berserk costs half the starting clock"
    );
    assert_eq!(
        berserked.black_time_left, first[0].black_time_left,
        "the opponent's clock is untouched"
    );

    // Once a move has been made it is too late to declare.
    use db_lib::schema::games;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    diesel::update(games::table.find(first[1].id))
        .set(games::turn.eq(3))
        .execute(&mut conn)
        .await
        .expect("advance the other game");
    let started = db_lib::models::Game::find_by_uuid(&first[1].id, &mut conn)
        .await
        .expect("reload");
    assert!(
        started.berserk(Color::White, &mut conn).await.is_err(),
        "berserk cannot be declared once the game is under way"
    );

    // Both games have to run long enough to count: lichess pays the berserk
    // bonus only from 14 moves, so winning instantly after berserking earns
    // nothing.
    for game in [&berserked, &started] {
        diesel::update(games::table.find(game.id))
            .set(games::turn.eq(30))
            .execute(&mut conn)
            .await
            .expect("give the game a realistic length");
    }
    let berserked = db_lib::models::Game::find_by_uuid(&berserked.id, &mut conn)
        .await
        .expect("reload the berserked game");

    // The berserked win is worth more than the plain one.
    finish_at(
        &berserked,
        TournamentGameResult::Winner(Color::White),
        60,
        started_at,
        &mut conn,
    )
    .await;
    finish_at(
        &started,
        TournamentGameResult::Winner(Color::White),
        61,
        started_at,
        &mut conn,
    )
    .await;

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    let brave = score(&standings, berserked.white_id, Tiebreaker::RawPoints);
    let plain = score(&standings, started.white_id, Tiebreaker::RawPoints);
    assert_eq!(
        score(&standings, berserked.white_id, Tiebreaker::Berserks),
        1.0,
        "the berserk is counted"
    );
    assert!(
        brave > plain,
        "a berserked win ({brave}) should beat a plain one ({plain})"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_paused_player_is_not_paired_until_they_resume() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(3), &mut conn).await;
    start(&mut fixture, &mut conn).await;

    let resting = fixture.players[0].id;
    fixture
        .tournament
        .pause_in_arena(&resting, &mut conn)
        .await
        .expect("step out of the pool");

    // Three players, one resting: the other two are paired and the paused one
    // sits out rather than being the odd one left over.
    let ProgressOutcome::Advanced(paired) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the two active players should be paired");
    };
    assert_eq!(paired.len(), 1);
    assert!(
        paired[0].white_id != resting && paired[0].black_id != resting,
        "a paused player must not be paired"
    );

    // Pausing twice is refused rather than written, because a second pause
    // would make every later replay fail.
    assert!(
        fixture
            .tournament
            .pause_in_arena(&resting, &mut conn)
            .await
            .is_err(),
        "a paused player cannot pause again"
    );

    fixture
        .tournament
        .resume_in_arena(&resting, &mut conn)
        .await
        .expect("step back into the pool");

    // Still nobody to play: the other two are mid-game.
    assert!(matches!(
        fixture.tournament.progress(&mut conn).await.unwrap(),
        ProgressOutcome::Waiting
    ));

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 3);
    assert_eq!(
        score(&standings, resting, Tiebreaker::GamesPlayed),
        0.0,
        "a player who rested the whole time has played nothing"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pausing_mid_game_lets_that_game_finish_and_still_count() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };

    // A player asks for a break while still playing. The break starts when the
    // game ends; the result stands either way.
    let leaving = first[0].white_id;
    fixture
        .tournament
        .pause_in_arena(&leaving, &mut conn)
        .await
        .expect("request a break mid-game");

    for (index, game) in first.iter().enumerate() {
        finish_at(
            game,
            TournamentGameResult::Winner(Color::White),
            60 + index as i64,
            started_at,
            &mut conn,
        )
        .await;
    }

    let standings = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&standings, 4);
    assert_eq!(
        score(&standings, leaving, Tiebreaker::RawPoints),
        1.0,
        "the game they were already playing still counts"
    );
    assert_eq!(score(&standings, leaving, Tiebreaker::GamesPlayed), 1.0);

    // And now that it is over, the break is in force.
    progress_and_assert_unpaired(
        &fixture,
        leaving,
        "the break took effect once their game ended",
        &mut conn,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_withdrawn_player_keeps_their_points_but_is_never_paired_again() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };
    for (index, game) in first.iter().enumerate() {
        finish_at(
            game,
            TournamentGameResult::Winner(Color::White),
            60 + index as i64,
            started_at,
            &mut conn,
        )
        .await;
    }

    let quitter = first[0].white_id;
    let before = standings_of(&fixture, &mut conn).await;
    let earned = score(&before, quitter, Tiebreaker::RawPoints);
    assert_eq!(earned, 1.0, "they won their game before leaving");

    fixture
        .tournament
        .withdraw_from_arena(&quitter, &mut conn)
        .await
        .expect("leave the arena for good");

    let after = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&after, 4);
    assert_eq!(
        score(&after, quitter, Tiebreaker::RawPoints),
        earned,
        "what they scored before leaving still stands"
    );

    // Three remain, so exactly one new game is possible and it cannot involve
    // the player who left.
    progress_and_assert_unpaired(
        &fixture,
        quitter,
        "a withdrawn player is never paired again",
        &mut conn,
    )
    .await;

    // Withdrawal is final: it cannot be undone by resuming.
    assert!(
        fixture
            .tournament
            .resume_in_arena(&quitter, &mut conn)
            .await
            .is_err(),
        "withdrawal is for good"
    );
    assert!(
        fixture
            .tournament
            .withdraw_from_arena(&quitter, &mut conn)
            .await
            .is_err(),
        "and cannot be repeated"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_account_stops_being_paired_but_keeps_what_it_scored() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };
    for (index, game) in first.iter().enumerate() {
        finish_at(
            game,
            TournamentGameResult::Winner(Color::White),
            60 + index as i64,
            started_at,
            &mut conn,
        )
        .await;
    }

    // A winner deletes their account. Their point stands, but nothing should
    // ever pair them again.
    let gone = first[0].white_id;
    let earned = score(
        &standings_of(&fixture, &mut conn).await,
        gone,
        Tiebreaker::RawPoints,
    );
    assert_eq!(earned, 1.0);

    let user = db_lib::models::User::find_by_uuid(&gone, &mut conn)
        .await
        .expect("load the leaving user");
    user.soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete");

    let after = standings_of(&fixture, &mut conn).await;
    assert_well_formed(&after, 4);
    assert_eq!(
        score(&after, gone, Tiebreaker::RawPoints),
        earned,
        "a deleted account keeps what it already scored"
    );

    progress_and_assert_unpaired(
        &fixture,
        gone,
        "a deleted account must never be paired again",
        &mut conn,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_or_invite_only_arena_refuses_a_late_joiner() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    // Two seats, both taken at start.
    let mut fixture = create_tournament(TournamentMode::Arena, full_arena(2), &mut conn).await;
    start(&mut fixture, &mut conn).await;

    let hopeful = create_user("too_late", 1800.0, &mut conn).await;
    assert!(
        fixture
            .tournament
            .join_arena(&hopeful.id, &mut conn)
            .await
            .is_err(),
        "joining late still cannot exceed the seat count"
    );

    // And an existing entrant cannot join twice and take a second seat.
    assert!(
        fixture
            .tournament
            .join_arena(&fixture.players[0].id, &mut conn)
            .await
            .is_err(),
        "a player already in the arena cannot join again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invite_only_arena_refuses_an_uninvited_late_joiner() {
    use db_lib::schema::tournaments;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    // Built open so the field can join, then closed — an invite-only
    // tournament refuses `join` outright, so a fixture cannot be built as one.
    let mut fixture = create_tournament(TournamentMode::Arena, arena(2), &mut conn).await;
    start(&mut fixture, &mut conn).await;

    diesel::update(tournaments::table.find(fixture.tournament.id))
        .set(tournaments::invite_only.eq(true))
        .execute(&mut conn)
        .await
        .expect("close the arena");
    fixture.tournament = db_lib::models::Tournament::find(fixture.tournament.id, &mut conn)
        .await
        .expect("reload tournament");

    let hopeful = create_user("uninvited", 1800.0, &mut conn).await;
    assert!(
        fixture
            .tournament
            .join_arena(&hopeful.id, &mut conn)
            .await
            .is_err(),
        "an uninvited player cannot join an invite-only arena, even with seats free"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn berserk_is_refused_outside_an_arena() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut swiss = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 2,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let games = start(&mut swiss, &mut conn).await;

    assert!(
        games[0].berserk(Color::White, &mut conn).await.is_err(),
        "halving a clock only makes sense where it buys arena points"
    );
}

/// A paired-but-never-started game has no clock — `compute_timeout_at` gives an
/// unstarted game no `timeout_at`, so the sweeper never sees it. If withdrawal
/// left one behind it would stay in flight forever and the arena could never
/// finish, however long its clock had run out.
#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_does_not_leave_an_unstarted_game_hanging_the_arena() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(2), &mut conn).await;
    start(&mut fixture, &mut conn).await;

    let ProgressOutcome::Advanced(paired) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair its two starters");
    };
    assert_eq!(paired.len(), 1);
    assert!(
        paired[0].timeout_at.is_none(),
        "an unstarted game has no clock to run out — that is the whole problem"
    );

    // One of them walks off before making a move.
    let leaving = paired[0].white_id;
    fixture
        .tournament
        .withdraw_player(&leaving, &leaving, &mut conn)
        .await
        .expect("withdraw");

    let games = games_of(&fixture, &mut conn).await;
    assert!(
        games.iter().all(|game| game.finished),
        "their untouched game is forfeited rather than left in flight"
    );

    // Run the clock out. Everything moves back by the same amount — shifting
    // only `started_at` would re-date the existing game relative to the arena
    // clock and the replay would rightly complain.
    use db_lib::schema::{
        games as games_table,
        tournament_arena_events,
        tournaments,
        tournaments_users,
    };
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    let shift = chrono::Duration::seconds(7200);
    diesel::update(tournaments::table.find(fixture.tournament.id))
        .set(tournaments::started_at.eq(fixture.tournament.started_at.map(|at| at - shift)))
        .execute(&mut conn)
        .await
        .expect("age the arena past its duration");
    diesel::update(
        tournaments_users::table.filter(tournaments_users::tournament_id.eq(fixture.tournament.id)),
    )
    .set(tournaments_users::joined_at.eq(tournaments_users::joined_at.nullable() - shift))
    .execute(&mut conn)
    .await
    .expect("age the joins with it");
    diesel::update(
        tournament_arena_events::table
            .filter(tournament_arena_events::tournament_id.eq(fixture.tournament.id)),
    )
    .set(tournament_arena_events::at.eq(tournament_arena_events::at - shift))
    .execute(&mut conn)
    .await
    .expect("age the withdrawal with it");
    for game in games_of(&fixture, &mut conn).await {
        diesel::update(games_table::table.find(game.id))
            .set((
                games_table::created_at.eq(game.created_at - shift),
                games_table::updated_at.eq(game.updated_at - shift),
            ))
            .execute(&mut conn)
            .await
            .expect("age the game with it");
    }

    let outcome = fixture
        .tournament
        .progress(&mut conn)
        .await
        .expect("tick the expired arena");
    assert!(
        matches!(outcome, ProgressOutcome::ReadyToFinish),
        "an expired arena with nothing in flight is over, got {outcome:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn breaks_are_rejected_outside_a_running_arena() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    // Not an arena at all.
    let mut swiss = create_tournament(
        TournamentMode::DutchSwiss,
        TournamentOpts {
            player_count: 4,
            rounds: 2,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    start(&mut swiss, &mut conn).await;
    assert!(
        swiss
            .tournament
            .pause_in_arena(&swiss.players[0].id, &mut conn)
            .await
            .is_err(),
        "only an arena has a pairing pool to step out of"
    );

    // An arena that has not started yet.
    let arena_fixture = create_tournament(TournamentMode::Arena, arena(2), &mut conn).await;
    assert!(
        arena_fixture
            .tournament
            .pause_in_arena(&arena_fixture.players[0].id, &mut conn)
            .await
            .is_err(),
        "there is nothing to pause before the clock starts"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn replaying_an_arena_twice_gives_the_same_standings() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let mut fixture = create_tournament(TournamentMode::Arena, arena(4), &mut conn).await;
    start(&mut fixture, &mut conn).await;
    let started_at = fixture.tournament.started_at.expect("arena has started");

    let ProgressOutcome::Advanced(first) = fixture.tournament.progress(&mut conn).await.unwrap()
    else {
        panic!("the arena should pair on its first tick");
    };
    for (index, game) in first.iter().enumerate() {
        finish_at(
            game,
            TournamentGameResult::Winner(Color::White),
            45 + index as i64 * 5,
            started_at,
            &mut conn,
        )
        .await;
    }
    fixture.tournament.progress(&mut conn).await.unwrap();

    // Standings are recomputed from scratch every call, by replaying the whole
    // arena. Two calls must agree, or the replay is not deterministic.
    let once = standings_of(&fixture, &mut conn).await;
    let twice = standings_of(&fixture, &mut conn).await;
    assert_eq!(
        once, twice,
        "an arena replay must be deterministic, or standings would flicker"
    );
    assert_well_formed(&once, 4);
}
