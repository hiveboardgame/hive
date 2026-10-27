mod common;

use chrono::{Duration, Utc};
use common::{
    arena::{elapse_rematch_wait, finish_by_resignation, start_arena_fixture},
    db::test_db,
    locks::{wait_for_backend_lock, Backend},
    tournament::create_user,
};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    get_conn,
    models::{TournamentFinalOutcome, TournamentUser},
    schema::{tournaments, tournaments_users},
    tournaments::arena,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{Position, Turn};
use shared_types::tournament::arena::PairingIntent;

#[tokio::test(flavor = "multi_thread")]
async fn pause_after_arena_completion_is_harmless_for_participants() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, players, games) = start_arena_fixture("finished_pause", 2, &mut conn).await;
    finish_by_resignation(&games[0], &mut conn).await;
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::hours(1))))
        .execute(&mut conn)
        .await
        .unwrap();
    arena::finalize_due(tournament.id, &mut conn).await.unwrap();
    let frozen = TournamentFinalOutcome::load(tournament.id, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let before = tournaments_users::table
        .find((tournament.id, players[0]))
        .first::<TournamentUser>(&mut conn)
        .await
        .unwrap();

    for _ in 0..2 {
        assert!(!arena::set_pairing_intent(
            tournament.id,
            players[0],
            PairingIntent::Paused,
            &mut conn
        )
        .await
        .expect("a post-game Pause arriving after completion succeeds"));
    }
    let after = tournaments_users::table
        .find((tournament.id, players[0]))
        .first::<TournamentUser>(&mut conn)
        .await
        .unwrap();
    assert_eq!(after.arena_pairing_intent, before.arena_pairing_intent);
    assert_eq!(after.arena_waiting_since, before.arena_waiting_since);
    assert_eq!(
        TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap(),
        Some(frozen)
    );

    assert!(
        arena::set_pairing_intent(tournament.id, players[0], PairingIntent::Enabled, &mut conn)
            .await
            .is_err(),
        "completion must still prevent Resume"
    );
    let outsider = create_user("pause_outsider", &mut conn).await;
    assert!(
        arena::set_pairing_intent(tournament.id, outsider.id, PairingIntent::Paused, &mut conn)
            .await
            .is_err(),
        "the harmless late Pause still requires membership"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn repeated_pause_reports_no_change() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, players, _) = start_arena_fixture("repeat_pause", 2, &mut conn).await;
    assert!(
        arena::set_pairing_intent(tournament.id, players[0], PairingIntent::Paused, &mut conn)
            .await
            .unwrap()
    );
    assert!(!arena::set_pairing_intent(
        tournament.id,
        players[0],
        PairingIntent::Paused,
        &mut conn
    )
    .await
    .unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_and_pairing_serialize_on_the_player_membership() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    for pause_first in [true, false] {
        let (tournament, players, games) = start_arena_fixture(
            if pause_first {
                "pause_first"
            } else {
                "pair_first"
            },
            2,
            &mut conn,
        )
        .await;
        finish_by_resignation(&games[0], &mut conn).await;
        elapse_rematch_wait(tournament.id, &mut conn).await;
        let user_id = players[0];
        let tournament_id = tournament.id;
        let mut competing_conn = db.pool.get_owned().await.unwrap();
        let Backend { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
            .get_result(&mut competing_conn)
            .await
            .unwrap();
        let online = players.clone();

        let competing = conn
            .transaction::<_, DbError, _>(async |tc| {
                if pause_first {
                    arena::set_pairing_intent(tournament_id, user_id, PairingIntent::Paused, tc)
                        .await?;
                } else {
                    let wave = arena::pair_waiting_with_presence(
                        tournament_id,
                        move |_| online.iter().copied().collect(),
                        tc,
                    )
                    .await?;
                    assert_eq!(wave.new_games.len(), 1);
                }
                let competing = tokio::spawn(async move {
                    if pause_first {
                        let wave = arena::pair_waiting_with_presence(
                            tournament_id,
                            move |_| players.iter().copied().collect(),
                            &mut competing_conn,
                        )
                        .await?;
                        assert!(
                            wave.new_games.is_empty(),
                            "a committed pause excludes the player"
                        );
                    } else {
                        arena::set_pairing_intent(
                            tournament_id,
                            user_id,
                            PairingIntent::Paused,
                            &mut competing_conn,
                        )
                        .await?;
                    }
                    Ok::<_, DbError>(())
                });
                // Observe the actual lock wait before allowing the first operation to commit.
                wait_for_backend_lock(&db.pool, pid).await;
                Ok(competing)
            })
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), competing)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let membership = TournamentUser::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .unwrap()
            .into_iter()
            .find(|membership| membership.user_id == user_id)
            .unwrap();
        assert_eq!(
            membership.pairing_intent().unwrap(),
            Some(PairingIntent::Paused)
        );
        assert!(membership.arena_waiting_since.is_none());
        let active_games = tournament
            .games(&mut conn)
            .await
            .unwrap()
            .into_iter()
            .filter(|game| !game.finished)
            .count();
        assert_eq!(active_games, usize::from(!pause_first));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn only_black_can_berserk_after_white_has_moved() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (_, _, games) = start_arena_fixture("berserk_turn", 2, &mut conn).await;
    let game = &games[0];
    execute(
        game.id,
        Command::Move {
            user_id: game.white_id,
            expected_turn: Some(game.turn.try_into().unwrap()),
            turn: Turn::Move("wA1".parse().unwrap(), Position::initial_spawn_position()),
            compensation: 0.0,
        },
        &mut conn,
    )
    .await
    .expect("White plays their opening move");

    let white = execute(
        game.id,
        Command::Berserk {
            user_id: game.white_id,
        },
        &mut conn,
    )
    .await;
    assert!(matches!(white, Err(DbError::InvalidAction { .. })));
    let black = execute(
        game.id,
        Command::Berserk {
            user_id: game.black_id,
        },
        &mut conn,
    )
    .await
    .expect("Black can still berserk before their opening move");
    let Outcome::Applied { game, .. } = black else {
        panic!("Black's berserk updates the game")
    };
    assert!(!game.white_berserked);
    assert!(game.black_berserked);
}
