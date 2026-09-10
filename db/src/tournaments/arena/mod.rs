use crate::models::{Game, Tournament};
use uuid::Uuid;

#[derive(Debug)]
pub struct StartOutcome {
    pub tournament: Tournament,
    pub started_now: bool,
    pub new_games: Vec<Game>,
    pub removed_invitees: Vec<Uuid>,
}

#[derive(Debug)]
pub struct PairingOutcome {
    pub changed: bool,
    pub new_games: Vec<Game>,
}

#[derive(Debug)]
pub struct FinalizeOutcome {
    pub tournament: Tournament,
    pub finished_now: bool,
}

#[derive(Debug)]
pub struct JoinOutcome {
    pub joined_now: bool,
}

mod pairing;
mod projection;
mod queries;
mod settlement;
mod state;

pub use pairing::{
    join_with_presence,
    pair_waiting_with_presence,
    set_pairing_intent,
    start_scheduled_with_presence,
};
pub(crate) use projection::{arena_projection_for_snapshot, ArenaFactsProjection};
pub use queries::{
    cutoff_candidates,
    opening_deadline_candidates,
    ordinary_timeout_candidates,
    pairing_candidates,
    scheduled_start_candidates,
};
pub(crate) use settlement::settle_arena_game;
pub use settlement::{finalize_due, finalize_due_in_transaction};

#[cfg(test)]
use pairing::{choose_featured_game, RankedFeaturedGame};
#[cfg(test)]
use state::{facts_before, load_arena_state_for_read, ArenaTerminal};

#[cfg(test)]
mod tests {
    use shared_types::tournament_view::TournamentFormatResponse;

    use super::{
        choose_featured_game,
        facts_before,
        load_arena_state_for_read,
        settlement::finalize_due_with_clock,
        state::load_arena_scoring_games,
        ArenaTerminal,
        RankedFeaturedGame,
    };
    use crate::{
        db_error::DbError,
        game_command::{execute_at, Command, Outcome},
        get_conn,
        models::{Game, Tournament, TournamentFinalOutcome, TournamentUser},
        schema::{
            arena_game_results,
            games,
            tournament_final_arena_results,
            tournaments,
            tournaments_users,
        },
        test_support::{
            db::test_db,
            tournament::{
                create_rr_tournament_with_configuration,
                create_user,
                fixed_instant,
                insert_unfrozen_memberships,
            },
        },
        tournaments::{
            arena,
            public::{load_arena_player_stats, load_by_id},
            rating,
        },
        DbConn,
    };
    use chrono::{DateTime, Duration, Utc};
    use diesel::{prelude::*, update};
    use diesel_async::{AsyncConnection, RunQueryDsl};
    use hive_lib::{Color, GameControl, Position, Turn};
    use shared_types::{
        tournament::{
            arena::{Config as ArenaConfig, PairingIntent},
            standings::Value,
            BotAdmission,
            Config,
            FormatConfig,
            RealtimeClock,
        },
        Conclusion,
        GameId,
        TournamentStatus,
    };
    use std::{
        collections::{HashMap, HashSet},
        num::NonZeroU32,
    };
    use uuid::Uuid;

    #[test]
    fn featured_game_policy_is_strict_and_ordinal_deterministic() {
        let current = Uuid::from_u128(1);
        let equal_earlier = RankedFeaturedGame {
            game_id: Uuid::from_u128(2),
            priority: 4,
            ordinal: 10,
        };
        let equal_later = RankedFeaturedGame {
            game_id: Uuid::from_u128(3),
            priority: 4,
            ordinal: 11,
        };
        let better = RankedFeaturedGame {
            game_id: Uuid::from_u128(4),
            priority: 3,
            ordinal: 12,
        };

        assert_eq!(
            choose_featured_game(Some((current, 4, true)), equal_later, &[equal_earlier],),
            current
        );
        assert_eq!(
            choose_featured_game(Some((current, 4, true)), equal_earlier, &[better]),
            better.game_id
        );
        assert_eq!(
            choose_featured_game(Some((current, 1, false)), equal_later, &[equal_earlier],),
            equal_earlier.game_id
        );
        assert_eq!(
            choose_featured_game(None, equal_later, &[equal_earlier]),
            equal_earlier.game_id
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn arena_player_and_game_commands_complete_while_tournament_is_locked() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("arena_rating_org", &mut conn).await;
        let players = [
            create_user("arena_rating_white", &mut conn).await.id,
            create_user("arena_rating_black", &mut conn).await.id,
        ];
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "arena_rating_refresh",
            TournamentStatus::NotStarted,
            Some(Utc::now() + Duration::hours(1)),
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Arena(ArenaConfig {
                    duration_seconds: NonZeroU32::new(3_600).unwrap(),
                    game_clock: RealtimeClock {
                        base_seconds: NonZeroU32::new(180).unwrap(),
                        increment_seconds: 1,
                    },
                }),
            },
            &mut conn,
        )
        .await;
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::seconds(1))))
            .execute(&mut conn)
            .await
            .expect("make Arena tournament due");
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let started = arena::start_scheduled_with_presence(
            tournament.id,
            move |_| players.into_iter().collect(),
            &mut conn,
        )
        .await
        .expect("start Arena");
        let game = started
            .new_games
            .into_iter()
            .next()
            .expect("materialize the initial Arena game");

        let tournament_id = tournament.id;
        let game_id = game.id;
        let white_id = game.white_id;
        let black_id = game.black_id;
        let terminal_at = Utc::now();
        let intent_pool = db.pool.clone();
        let command_pool = db.pool.clone();
        let terminal = conn
            .transaction::<_, DbError, _>(async move |tc| {
                Tournament::find_for_update(tournament_id, tc).await?;
                let intent = tokio::spawn(async move {
                    let mut intent_conn = get_conn(&intent_pool)
                        .await
                        .expect("get Arena intent command connection");
                    arena::set_pairing_intent(
                        tournament_id,
                        white_id,
                        PairingIntent::Paused,
                        &mut intent_conn,
                    )
                    .await
                });
                tokio::time::timeout(std::time::Duration::from_secs(2), intent)
                    .await
                    .expect("Arena pairing-intent command does not wait for the Tournament lock")
                    .expect("join Arena pairing-intent command")?;
                let command = tokio::spawn(async move {
                    let mut command_conn = get_conn(&command_pool)
                        .await
                        .expect("get Arena command connection");
                    execute_at(
                        game_id,
                        Command::Control {
                            user_id: black_id,
                            control: GameControl::Resign(Color::Black),
                        },
                        terminal_at,
                        &mut command_conn,
                    )
                    .await
                });
                tokio::time::timeout(std::time::Duration::from_secs(2), command)
                    .await
                    .expect("Arena terminal command does not wait for the Tournament lock")
                    .expect("join Arena terminal command")
            })
            .await
            .expect("commit Arena terminal command while Tournament is locked elsewhere");
        let Outcome::Applied { game: terminal, .. } = terminal else {
            panic!("resignation updates the Arena game")
        };
        let expected_white = rating::rounded(
            terminal.white_rating.expect("White completion rating")
                + terminal.white_rating_change.unwrap_or(0.0),
        )
        .expect("White completion rating stays in the supported domain");
        let expected_black = rating::rounded(
            terminal.black_rating.expect("Black completion rating")
                + terminal.black_rating_change.unwrap_or(0.0),
        )
        .expect("Black completion rating stays in the supported domain");
        assert_ne!(terminal.white_rating_change.unwrap_or(0.0), 0.0);
        assert_ne!(terminal.black_rating_change.unwrap_or(0.0), 0.0);

        let live_tournament = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload live Arena");
        let state = load_arena_state_for_read(live_tournament, &mut conn)
            .await
            .expect("reconstruct Arena after sealing");
        let (player_facts, terminal_facts) = facts_before(
            &state,
            terminal.finished_at.expect("Arena completion timestamp"),
            state.boundaries.ends_at,
        )
        .expect("reconstruct Arena facts");
        for (user_id, expected_rating) in [
            (game.white_id, expected_white),
            (game.black_id, expected_black),
        ] {
            let index = state
                .memberships
                .iter()
                .position(|membership| membership.user_id == user_id)
                .expect("Arena participant remains present");
            assert_eq!(state.memberships[index].arena_rating, Some(expected_rating));
            assert_eq!(player_facts[index].arena_rating, Some(expected_rating));
        }
        let white_membership = state
            .memberships
            .iter()
            .find(|membership| membership.user_id == game.white_id)
            .expect("White remains an Arena participant");
        assert_eq!(
            white_membership.pairing_intent().unwrap(),
            Some(PairingIntent::Paused)
        );
        assert_eq!(white_membership.arena_waiting_since, None);
        let black_membership = state
            .memberships
            .iter()
            .find(|membership| membership.user_id == game.black_id)
            .expect("Black remains an Arena participant");
        assert_eq!(
            black_membership.pairing_intent().unwrap(),
            Some(PairingIntent::Enabled)
        );
        assert_eq!(
            black_membership.arena_waiting_since, terminal.finished_at,
            "the opponent requeue is committed with the terminal Game",
        );
        assert_eq!(terminal_facts.len(), 1);
        assert_eq!(
            terminal_facts[0].native.white_rating,
            terminal.white_rating.and_then(rating::rounded)
        );
        assert_eq!(
            terminal_facts[0].native.black_rating,
            terminal.black_rating.and_then(rating::rounded)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn no_start_pauses_absent_player_and_requeues_opponent() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");

        for (suffix, white_moved) in [("wa", false), ("ba", true)] {
            let organizer = create_user(&format!("ans_{suffix}_org"), &mut conn).await;
            let players = [
                create_user(&format!("ans_{suffix}_first"), &mut conn)
                    .await
                    .id,
                create_user(&format!("ans_{suffix}_second"), &mut conn)
                    .await
                    .id,
                create_user(&format!("ans_{suffix}_waiter"), &mut conn)
                    .await
                    .id,
            ];
            let tournament = create_rr_tournament_with_configuration(
                organizer.id,
                &format!("arena_no_start_{suffix}"),
                TournamentStatus::NotStarted,
                Some(Utc::now() + Duration::hours(1)),
                Config {
                    bot_admission: BotAdmission::HumansAndBots,
                    format: FormatConfig::Arena(ArenaConfig {
                        duration_seconds: NonZeroU32::new(3_600).unwrap(),
                        game_clock: RealtimeClock {
                            base_seconds: NonZeroU32::new(180).unwrap(),
                            increment_seconds: 1,
                        },
                    }),
                },
                &mut conn,
            )
            .await;
            update(tournaments::table.find(tournament.id))
                .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::seconds(1))))
                .execute(&mut conn)
                .await
                .expect("make Arena tournament due");
            insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
            let online = move |_: &[Uuid]| players.into_iter().collect::<HashSet<_>>();
            let started = arena::start_scheduled_with_presence(tournament.id, online, &mut conn)
                .await
                .expect("start Arena");
            assert_eq!(started.new_games.len(), 1);
            assert_eq!(
                started.tournament.featured_game_id,
                Some(started.new_games[0].id)
            );
            let game = started.new_games.into_iter().next().unwrap();
            let waiter = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("load initial Arena waiting player")
                .into_iter()
                .find(|membership| membership.arena_waiting_since.is_some())
                .expect("odd Arena field leaves one player waiting")
                .user_id;
            let empty_wave = arena::pair_waiting_with_presence(
                tournament.id,
                move |_| players.into_iter().collect(),
                &mut conn,
            )
            .await
            .expect("visit Arena with no available pairing");
            assert!(empty_wave.new_games.is_empty());
            assert_eq!(
                Tournament::find(tournament.id, &mut conn)
                    .await
                    .expect("reload Arena after empty wave")
                    .featured_game_id,
                Some(game.id),
                "an empty wave must not change the feature"
            );

            if white_moved {
                execute_at(
                    game.id,
                    Command::Move {
                        user_id: game.white_id,
                        turn: Turn::Move(
                            "wA1".parse().expect("parse White ant"),
                            Position::initial_spawn_position(),
                        ),
                        compensation: 0.0,
                    },
                    Utc::now(),
                    &mut conn,
                )
                .await
                .expect("play White Arena opening move");
            }

            let no_start_at = DateTime::from_timestamp_micros(Utc::now().timestamp_micros())
                .expect("database-precision observation timestamp");
            let no_start_deadline = no_start_at - Duration::minutes(5);
            update(games::table.find(game.id))
                .set(games::arena_move_due_at.eq(Some(no_start_deadline)))
                .execute(&mut conn)
                .await
                .expect("make Arena opening deadline due");
            let settled = execute_at(game.id, Command::SettleDeadline, no_start_at, &mut conn)
                .await
                .expect("settle Arena no-start");
            let Outcome::Applied { game: terminal, .. } = &settled else {
                panic!("deadline settlement updates the Arena game")
            };
            let (absent, opponent) = if white_moved {
                assert_eq!(terminal.turn, 1);
                (game.black_id, game.white_id)
            } else {
                assert_eq!(terminal.turn, 0);
                (game.white_id, game.black_id)
            };
            let terminal_at = terminal.finished_at.expect("no-start finish timestamp");
            assert_eq!(terminal_at, no_start_deadline);
            assert_eq!(
                Tournament::find(tournament.id, &mut conn)
                    .await
                    .expect("reload Arena after no-start")
                    .featured_game_id,
                Some(game.id),
                "finishing a featured game alone must not clear it"
            );
            let public_snapshot = load_by_id(tournament.id, &mut conn)
                .await
                .expect("load public Arena snapshot with a featured no-start");
            let (_, patch_stats) = load_arena_player_stats(tournament.id, &mut conn)
                .await
                .expect("load live Arena patch stats");
            let TournamentFormatResponse::Arena {
                games: arena_projection_games,
                player_stats: arena_projection_player_stats,
                ..
            } = public_snapshot.format
            else {
                panic!("Arena snapshot has another format projection")
            };
            assert_eq!(
                patch_stats, arena_projection_player_stats,
                "live patch and snapshot statistics agree"
            );
            assert!(
                arena_projection_games
                    .iter()
                    .any(|candidate| candidate.game.game_id == GameId(game.nanoid.clone())),
                "a featured no-start remains resolvable in the public snapshot"
            );

            let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("reload Arena pairing state");
            let absent_membership = memberships
                .iter()
                .find(|membership| membership.user_id == absent)
                .expect("absent player remains an Arena member");
            assert_eq!(
                absent_membership.pairing_intent().unwrap(),
                Some(PairingIntent::Paused)
            );
            assert_eq!(absent_membership.arena_waiting_since, None);
            let opponent_membership = memberships
                .iter()
                .find(|membership| membership.user_id == opponent)
                .expect("opponent remains an Arena member");
            assert_eq!(
                opponent_membership.pairing_intent().unwrap(),
                Some(PairingIntent::Enabled)
            );
            assert_eq!(opponent_membership.arena_waiting_since, Some(no_start_at));

            let paired = arena::pair_waiting_with_presence(
                tournament.id,
                move |_| players.into_iter().collect(),
                &mut conn,
            )
            .await
            .expect("pair requeued opponent with waiting player");
            assert_eq!(paired.new_games.len(), 1);
            let next_game = &paired.new_games[0];
            assert_eq!(
                Tournament::find(tournament.id, &mut conn)
                    .await
                    .expect("reload Arena after replacement wave")
                    .featured_game_id,
                Some(next_game.id),
                "a later non-empty wave replaces the finished feature"
            );
            assert_eq!(
                [next_game.white_id, next_game.black_id]
                    .into_iter()
                    .collect::<HashSet<_>>(),
                [opponent, waiter].into_iter().collect::<HashSet<_>>()
            );
            assert_ne!(next_game.white_id, absent);
            assert_ne!(next_game.black_id, absent);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn late_scheduled_start_creates_no_games_before_worker_finalization() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("late_arena_org", &mut conn).await;
        let players = [
            create_user("late_arena_first", &mut conn).await.id,
            create_user("late_arena_second", &mut conn).await.id,
        ];
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "arena_late_scheduled_start",
            TournamentStatus::NotStarted,
            Some(Utc::now() + Duration::hours(1)),
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Arena(ArenaConfig {
                    duration_seconds: NonZeroU32::new(3_600).unwrap(),
                    game_clock: RealtimeClock {
                        base_seconds: NonZeroU32::new(180).unwrap(),
                        increment_seconds: 1,
                    },
                }),
            },
            &mut conn,
        )
        .await;
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::hours(2))))
            .execute(&mut conn)
            .await
            .expect("make the scheduled Arena start later than its cutoff");
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;

        let started = arena::start_scheduled_with_presence(
            tournament.id,
            move |_| players.into_iter().collect(),
            &mut conn,
        )
        .await
        .expect("start the overdue Arena");
        assert!(started.started_now);
        assert!(started.new_games.is_empty());
        assert_eq!(started.tournament.status(), TournamentStatus::InProgress);
        assert!(
            TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("reload late-start Arena memberships")
                .iter()
                .all(|membership| membership.arena_waiting_since.is_none())
        );
        assert!(!arena::pairing_candidates(None, Utc::now(), 100, &mut conn)
            .await
            .expect("load Arena pairing candidates")
            .contains(&tournament.id));
        assert!(arena::cutoff_candidates(Utc::now(), 100, &mut conn)
            .await
            .expect("load Arena cutoff candidates")
            .contains(&tournament.id));

        let finalized = arena::finalize_due(tournament.id, &mut conn)
            .await
            .expect("deadline worker finalizes the overdue Arena");
        assert!(finalized.finished_now);
        assert_eq!(finalized.tournament.status(), TournamentStatus::Finished);
        assert!(finalized
            .tournament
            .games(&mut conn)
            .await
            .expect("load games from late-start Arena")
            .is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_pairing_creates_at_most_one_active_game_per_player() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, players, games) = start_arena_fixture("pair_lock", 2, &mut conn).await;
        let game = games.into_iter().next().expect("initial Arena Game");
        finish_by_resignation(&game, Utc::now(), &mut conn).await;

        let first_pool = db.pool.clone();
        let second_pool = db.pool.clone();
        let first_players = players.clone();
        let second_players = players.clone();
        let tournament_id = tournament.id;
        let first = tokio::spawn(async move {
            let mut pairing_conn = get_conn(&first_pool)
                .await
                .expect("get first Arena pairing connection");
            arena::pair_waiting_with_presence(
                tournament_id,
                move |_| first_players.iter().copied().collect(),
                &mut pairing_conn,
            )
            .await
        });
        let second = tokio::spawn(async move {
            let mut pairing_conn = get_conn(&second_pool)
                .await
                .expect("get second Arena pairing connection");
            arena::pair_waiting_with_presence(
                tournament_id,
                move |_| second_players.iter().copied().collect(),
                &mut pairing_conn,
            )
            .await
        });
        let first = first
            .await
            .expect("join first Arena pairing")
            .expect("first Arena pairing succeeds");
        let second = second
            .await
            .expect("join second Arena pairing")
            .expect("second Arena pairing succeeds");
        assert_eq!(first.new_games.len() + second.new_games.len(), 1);

        let active = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload concurrently paired Arena")
            .games(&mut conn)
            .await
            .expect("load concurrently paired Arena Games")
            .into_iter()
            .filter(|game| !game.finished)
            .collect::<Vec<_>>();
        assert_eq!(active.len(), 1);
        assert_eq!(
            [active[0].white_id, active[0].black_id]
                .into_iter()
                .collect::<HashSet<_>>(),
            players.into_iter().collect(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deadline_finalizer_does_not_wait_for_locked_arena_game() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, _, games) = start_arena_fixture("final_lock", 2, &mut conn).await;
        let game = games.into_iter().next().expect("initial Arena Game");
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::hours(2))))
            .execute(&mut conn)
            .await
            .expect("make Arena finalization due");

        let tournament_id = tournament.id;
        let game_id = game.id;
        let finalizer_pool = db.pool.clone();
        let finalized = conn
            .transaction::<_, DbError, _>(async move |tc| {
                Game::find_by_uuid_for_update(&game_id, tc).await?;
                let finalizer = tokio::spawn(async move {
                    let mut finalizer_conn = get_conn(&finalizer_pool)
                        .await
                        .expect("get Arena finalizer connection");
                    arena::finalize_due(tournament_id, &mut finalizer_conn).await
                });
                tokio::time::timeout(std::time::Duration::from_secs(2), finalizer)
                    .await
                    .expect("Arena finalizer does not wait for the Game lock")
                    .expect("join Arena finalizer")
            })
            .await
            .expect("finalize Arena while its Game is locked elsewhere");
        assert!(finalized.finished_now);
        assert_eq!(finalized.tournament.status(), TournamentStatus::Finished);
        assert!(
            !Game::find_by_uuid(&game.id, &mut conn)
                .await
                .expect("reload Game left active by Arena finalization")
                .finished
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_rechecks_cutoff_after_waiting_for_tournament_lock() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, players, games) = start_arena_fixture("cutoff_lock", 2, &mut conn).await;
        let game = games.into_iter().next().expect("initial Arena Game");
        finish_by_resignation(&game, Utc::now(), &mut conn).await;

        let tournament_id = tournament.id;
        let pairing_offset = Duration::seconds(3_540);
        let initially_open_until = Utc::now() + Duration::hours(1);
        update(tournaments::table.find(tournament_id))
            .set(tournaments::starts_at.eq(Some(initially_open_until - pairing_offset)))
            .execute(&mut conn)
            .await
            .expect("keep Arena pairing initially open");
        let pairing_pool = db.pool.clone();
        let present = players.clone();
        let pairing_task = conn
            .transaction::<_, DbError, _>(async move |tc| {
                Tournament::find_for_update(tournament_id, tc).await?;
                let (started_tx, started_rx) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(async move {
                    let mut pairing_conn = get_conn(&pairing_pool)
                        .await
                        .expect("get Arena pairing connection");
                    let _ = started_tx.send(());
                    arena::pair_waiting_with_presence(
                        tournament_id,
                        move |_| present.iter().copied().collect(),
                        &mut pairing_conn,
                    )
                    .await
                });
                started_rx.await.expect("start the blocked Arena pairing");
                let closed_at = Utc::now() - Duration::seconds(1);
                update(tournaments::table.find(tournament_id))
                    .set(tournaments::starts_at.eq(Some(closed_at - pairing_offset)))
                    .execute(tc)
                    .await?;
                Ok(task)
            })
            .await
            .expect("release Tournament lock after pairing cutoff");
        let outcome = pairing_task
            .await
            .expect("join Arena pairing task")
            .expect("recheck Arena pairing cutoff");
        assert!(!outcome.changed);
        assert!(outcome.new_games.is_empty());
        assert_eq!(
            Tournament::find(tournament.id, &mut conn)
                .await
                .expect("reload cutoff Arena")
                .games(&mut conn)
                .await
                .expect("load cutoff Arena Games")
                .len(),
            1,
            "a pairing request queued before cutoff cannot materialize after cutoff",
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn post_cutoff_game_can_finish_without_changing_final_outcome() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("arena_cancel_org", &mut conn).await;
        let players = [
            create_user("arena_cancel_first", &mut conn).await.id,
            create_user("arena_cancel_second", &mut conn).await.id,
            create_user("arena_cancel_third", &mut conn).await.id,
            create_user("arena_cancel_fourth", &mut conn).await.id,
            create_user("arena_cancel_fifth", &mut conn).await.id,
        ];
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "arena_cancelled_game_finish",
            TournamentStatus::NotStarted,
            Some(Utc::now() + Duration::hours(1)),
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Arena(ArenaConfig {
                    duration_seconds: NonZeroU32::new(3_600).unwrap(),
                    game_clock: RealtimeClock {
                        base_seconds: NonZeroU32::new(180).unwrap(),
                        increment_seconds: 1,
                    },
                }),
            },
            &mut conn,
        )
        .await;
        let due_at =
            DateTime::from_timestamp_micros((Utc::now() - Duration::seconds(1)).timestamp_micros())
                .expect("due timestamp");
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(due_at)))
            .execute(&mut conn)
            .await
            .expect("make Arena tournament due");
        insert_unfrozen_memberships(tournament.id, &players[..3], fixed_instant(), &mut conn).await;
        let online = move |_: &[Uuid]| players[..3].iter().copied().collect::<HashSet<_>>();
        let started = arena::start_scheduled_with_presence(tournament.id, online, &mut conn)
            .await
            .expect("start Arena");
        let mut games = started.new_games;
        let initial_waiter = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("reload Arena memberships")
            .into_iter()
            .find(|membership| membership.arena_waiting_since.is_some())
            .expect("odd Arena field leaves one entrant waiting")
            .user_id;
        let joined = arena::join_with_presence(
            tournament.id,
            players[3],
            move |_| [initial_waiter, players[3]].into_iter().collect(),
            &mut conn,
        )
        .await
        .expect("live Join enqueues the new entrant");
        assert!(joined.joined_now);
        let paired = arena::pair_waiting_with_presence(
            tournament.id,
            move |_| [initial_waiter, players[3]].into_iter().collect(),
            &mut conn,
        )
        .await
        .expect("the Arena worker pairs the enqueued entrant");
        assert_eq!(paired.new_games.len(), 1);
        games.extend(paired.new_games);
        let waiting = arena::join_with_presence(
            tournament.id,
            players[4],
            move |_| [players[4]].into_iter().collect(),
            &mut conn,
        )
        .await
        .expect("live Join may leave an unmatched entrant waiting");
        assert!(waiting.joined_now);
        games.sort_by_key(|game| game.arena_ordinal);
        assert_eq!(
            games
                .iter()
                .map(|game| game.arena_ordinal)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)],
            "Arena reconstruction order is the persisted ordinal order",
        );
        let no_start_game = games.remove(0);
        let late_game = games.remove(0);

        let no_start_at = Utc::now();
        update(games::table.find(no_start_game.id))
            .set(games::arena_move_due_at.eq(Some(no_start_at - Duration::milliseconds(1))))
            .execute(&mut conn)
            .await
            .expect("make the first Arena opening deadline due");
        let no_start = execute_at(
            no_start_game.id,
            Command::SettleDeadline,
            no_start_at,
            &mut conn,
        )
        .await
        .expect("settle Arena no-start");
        assert!(matches!(
            no_start,
            Outcome::Applied { game, newly_terminal: true, .. }
                if game.finished && game.turn == 0
        ));

        let live_tournament = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload live Arena");
        let state = load_arena_state_for_read(live_tournament, &mut conn)
            .await
            .expect("reconstruct Arena in ordinal order");
        assert!(state
            .games
            .windows(2)
            .all(|pair| pair[0].ordinal < pair[1].ordinal));
        let mut exact_cutoff = state;
        let ends_at = exact_cutoff.boundaries.ends_at;
        let (terminal_result, terminal_ratings) = exact_cutoff
            .games
            .iter()
            .find_map(|game| {
                game.terminal
                    .map(|terminal| (terminal.result, terminal.ratings))
            })
            .expect("existing Arena terminal result");
        let excluded = exact_cutoff
            .games
            .iter_mut()
            .find(|game| game.game.id == late_game.id)
            .unwrap();
        excluded.game.finished = true;
        excluded.game.finished_at = Some(ends_at);
        excluded.game.conclusion = Conclusion::Draw.to_string();
        excluded.terminal = Some(ArenaTerminal {
            result: terminal_result,
            at: ends_at,
            ratings: terminal_ratings,
        });
        let (_, exact_cutoff_terminals) =
            facts_before(&exact_cutoff, ends_at, ends_at).expect("reconstruct exact-cutoff facts");
        assert_eq!(
            exact_cutoff_terminals.len(),
            1,
            "a terminal exactly at cutoff is excluded"
        );

        let post_cutoff_move_at = late_game.created_at + Duration::seconds(1);
        let cutoff_starts_at = post_cutoff_move_at - Duration::seconds(3_601);
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(cutoff_starts_at)))
            .execute(&mut conn)
            .await
            .expect("move Arena past cutoff");
        update(tournaments_users::table.filter(tournaments_users::tournament_id.eq(tournament.id)))
            .set(tournaments_users::arena_waiting_since.eq(None::<DateTime<Utc>>))
            .execute(&mut conn)
            .await
            .expect("clear reconstructed waiting state before the cutoff fixture");
        update(tournaments_users::table.find((tournament.id, players[4])))
            .set((
                tournaments_users::accepted_at.eq(cutoff_starts_at + Duration::minutes(1)),
                tournaments_users::arena_waiting_since
                    .eq(Some(cutoff_starts_at + Duration::minutes(2))),
            ))
            .execute(&mut conn)
            .await
            .expect("retain one participant waiting since after Arena start");
        let game_count = tournament
            .games(&mut conn)
            .await
            .expect("count Arena games")
            .len();
        let rejected_join = arena::join_with_presence(
            tournament.id,
            players[0],
            move |_| players.into_iter().collect::<HashSet<_>>(),
            &mut conn,
        )
        .await;
        assert!(matches!(
            rejected_join,
            Err(DbError::InvalidAction { ref info })
                if info == "Arena Join is closed at the pairing cutoff"
        ));
        assert_eq!(
            tournament
                .games(&mut conn)
                .await
                .expect("recount Arena games")
                .len(),
            game_count,
        );
        let pairing_after_cutoff =
            arena::pair_waiting_with_presence(tournament.id, |_| HashSet::new(), &mut conn)
                .await
                .expect("visit Arena pairing after cutoff");
        assert!(!pairing_after_cutoff.changed);
        assert!(pairing_after_cutoff.new_games.is_empty());
        assert_eq!(
            Tournament::find(tournament.id, &mut conn)
                .await
                .expect("reload Arena after cutoff pairing")
                .status(),
            TournamentStatus::InProgress,
            "pairing must not own Arena finalization",
        );
        assert_eq!(
            TournamentFinalOutcome::load(tournament.id, &mut conn)
                .await
                .expect("check outcome before worker finalization"),
            None,
        );
        let finalized = arena::finalize_due(tournament.id, &mut conn)
            .await
            .expect("deadline worker finalizes Arena at cutoff");
        assert!(finalized.finished_now);
        let frozen = TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .expect("load outcome")
            .expect("final outcome");
        let repeated = arena::finalize_due(tournament.id, &mut conn)
            .await
            .expect("repeat Arena finalization");
        assert!(!repeated.finished_now);

        let post_cutoff_move = execute_at(
            late_game.id,
            Command::Move {
                user_id: late_game.white_id,
                turn: Turn::Move(
                    "wA1".parse().expect("parse White ant"),
                    Position::initial_spawn_position(),
                ),
                compensation: 0.0,
            },
            post_cutoff_move_at,
            &mut conn,
        )
        .await
        .expect("play the Arena game after standings freeze");
        assert!(matches!(
            post_cutoff_move,
            Outcome::Applied {
                ref game,
                ..
            } if game.turn == 1 && !game.finished
        ));

        let Outcome::Applied {
            game: post_cutoff_game,
            ..
        } = &post_cutoff_move
        else {
            panic!("Arena move updates the game")
        };
        let deadline = post_cutoff_game
            .arena_move_due_at
            .expect("Black has a post-move Arena opening deadline")
            + Duration::seconds(1);
        let terminal = execute_at(late_game.id, Command::SettleDeadline, deadline, &mut conn)
            .await
            .expect("settle cancelled game");
        assert!(matches!(
            terminal,
            Outcome::Applied {
                game,
                newly_terminal: true,
                ..
            } if game.finished
        ));
        assert_eq!(
            TournamentFinalOutcome::load(tournament.id, &mut conn)
                .await
                .expect("reload outcome"),
            Some(frozen)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finished_projection_excludes_disjoint_ordinal_tail_games() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, players, first_wave) =
            start_arena_fixture("arena_tail", 4, &mut conn).await;

        for game in &first_wave {
            finish_by_resignation(game, game.created_at, &mut conn).await;
        }
        let present = players.clone();
        let second_wave = arena::pair_waiting_with_presence(
            tournament.id,
            move |_| present.iter().copied().collect(),
            &mut conn,
        )
        .await
        .expect("pair the second Arena wave")
        .new_games;
        assert_eq!(second_wave.len(), 2);

        let cutoff = second_wave
            .iter()
            .map(|game| game.created_at)
            .max()
            .expect("second wave has a creation instant")
            + Duration::microseconds(1);
        let frozen = freeze_arena_at(tournament.id, cutoff, &mut conn).await;
        assert_eq!(
            included_arena_game_ids(tournament.id, &mut conn)
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>(),
            first_wave
                .iter()
                .map(|game| game.id)
                .collect::<HashSet<_>>(),
        );
        for game in &second_wave {
            finish_by_resignation(game, game.created_at, &mut conn).await;
        }

        let finished = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload finished Arena");
        let snapshot = load_by_id(finished.id, &mut conn)
            .await
            .expect("rebuild finished Arena projection");
        assert_eq!(snapshot.standings, Some(frozen.standings.clone()));
        let (_, patch_stats) = load_arena_player_stats(tournament.id, &mut conn)
            .await
            .expect("load Arena patch stats");
        let TournamentFormatResponse::Arena {
            games: projected_games,
            player_stats: projected_player_stats,
            ..
        } = snapshot.format
        else {
            panic!("finished Arena has another format projection")
        };
        assert_eq!(
            patch_stats, projected_player_stats,
            "patch and snapshot preserve the same frozen statistics"
        );
        assert_eq!(
            projected_games
                .iter()
                .map(|game| game.ordinal)
                .collect::<Vec<_>>(),
            vec![0, 1],
            "only the two results visible to finalization remain in Arena history",
        );
        assert!(projected_player_stats
            .iter()
            .all(|stats| stats.games_scored == 1));

        let frozen_ratings = frozen
            .arena_ratings
            .unwrap()
            .into_iter()
            .map(|rating| (rating.user_id, Some(rating.rating)))
            .collect::<HashMap<_, _>>();
        for stats in &projected_player_stats {
            assert_eq!(stats.arena_rating, frozen_ratings[&stats.player]);
        }
        for game in second_wave {
            let durable = Game::find_by_uuid(&game.id, &mut conn)
                .await
                .expect("excluded tail Game remains durable");
            assert!(durable.finished);
            assert_ne!(durable.white_rating_change.unwrap_or(0.0), 0.0);
            assert_ne!(durable.black_rating_change.unwrap_or(0.0), 0.0);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finished_projection_excludes_no_start_without_an_inclusion_record() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, _, games) = start_arena_fixture("arena_ns_tail", 2, &mut conn).await;
        let game = games.into_iter().next().expect("initial Arena Game");
        update(games::table.find(game.id))
            .set(games::arena_move_due_at.eq(Some(game.created_at)))
            .execute(&mut conn)
            .await
            .expect("put the no-start deadline before cutoff");
        let cutoff = game.created_at + Duration::microseconds(1);
        let frozen = freeze_arena_at(tournament.id, cutoff, &mut conn).await;

        let outcome = execute_at(game.id, Command::SettleDeadline, game.created_at, &mut conn)
            .await
            .expect("commit the cutoff-racing no-start");
        assert!(matches!(
            outcome,
            Outcome::Applied {
                game,
                newly_terminal: true,
                ..
            } if game.finished && game.turn == 0
        ));

        let finished = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload finished Arena");
        let snapshot = load_by_id(finished.id, &mut conn)
            .await
            .expect("rebuild finished Arena after no-start race");
        assert_eq!(snapshot.standings, Some(frozen.standings));
        let (_, patch_stats) = load_arena_player_stats(tournament.id, &mut conn)
            .await
            .expect("load Arena patch stats");
        let TournamentFormatResponse::Arena {
            games: projected_games,
            player_stats: projected_player_stats,
            ..
        } = snapshot.format
        else {
            panic!("finished Arena has another format projection")
        };
        assert_eq!(
            patch_stats, projected_player_stats,
            "patch and snapshot preserve the same frozen statistics"
        );
        assert!(projected_games.is_empty());
        assert!(projected_player_stats
            .iter()
            .all(|stats| stats.games_scored == 0));
        let durable = Game::find_by_uuid(&game.id, &mut conn)
            .await
            .expect("excluded no-start Game remains durable");
        assert!(durable.finished);
        assert_ne!(durable.white_rating_change.unwrap_or(0.0), 0.0);
        assert_ne!(durable.black_rating_change.unwrap_or(0.0), 0.0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finalization_ignores_waiting_time_from_a_post_cutoff_completion() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let (tournament, players, games) = start_arena_fixture("final_wait", 2, &mut conn).await;
        let first = &games[0];
        finish_by_resignation(first, first.created_at, &mut conn).await;
        let first_points = arena_game_results::table
            .find(first.id)
            .select((
                arena_game_results::white_points,
                arena_game_results::black_points,
            ))
            .first::<(i64, i64)>(&mut conn)
            .await
            .unwrap();
        let late = arena::pair_waiting_with_presence(
            tournament.id,
            move |_| players.iter().copied().collect(),
            &mut conn,
        )
        .await
        .unwrap()
        .new_games
        .remove(0);
        let cutoff = late.created_at + Duration::seconds(1);
        let completed_at = cutoff + Duration::seconds(1);
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(cutoff - Duration::seconds(3_600))))
            .execute(&mut conn)
            .await
            .unwrap();
        finish_by_resignation(&late, completed_at, &mut conn).await;
        assert!(
            TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .unwrap()
                .iter()
                .all(|membership| membership.arena_waiting_since == Some(completed_at))
        );

        let finalized = finalize_due_with_clock(tournament.id, move || completed_at, &mut conn)
            .await
            .expect("finalize even when completion requeued players after the scoring cutoff");
        assert!(finalized.finished_now);
        assert_eq!(
            included_arena_game_ids(tournament.id, &mut conn)
                .await
                .unwrap(),
            vec![first.id],
        );
        let frozen = TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap()
            .unwrap();
        let points = frozen
            .standings
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .map(|row| {
                let Value::Score(points) = row.primary_score else {
                    panic!("Arena standings contain exact scores")
                };
                (row.user_id, i64::from(points.value()))
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(
            points,
            HashMap::from([
                (first.white_id, first_points.0),
                (first.black_id, first_points.1)
            ]),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scoring_history_selects_completed_games_for_either_color_in_ordinal_order() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let (tournament, _, games) = start_arena_fixture("award_history", 8, &mut conn).await;
        for game in games.iter().take(3) {
            finish_by_resignation(game, game.created_at, &mut conn).await;
        }
        for (players, expected) in [
            (
                [games[0].white_id, games[1].black_id],
                vec![games[0].id, games[1].id],
            ),
            ([games[0].black_id, games[3].white_id], vec![games[0].id]),
        ] {
            let history = load_arena_scoring_games(tournament.id, players, &mut conn)
                .await
                .unwrap();
            assert_eq!(
                history
                    .iter()
                    .map(|entry| entry.game.id)
                    .collect::<Vec<_>>(),
                expected,
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn award_write_failure_rolls_back_terminal_game_and_retry_awards_once() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, _, games) = start_arena_fixture("award_atomic", 2, &mut conn).await;
        let game = games.into_iter().next().unwrap();
        let rollback = conn.transaction::<(), DbError, _>(async |tc| {
            diesel::sql_query("ALTER TABLE arena_game_results ADD CONSTRAINT reject_test_award CHECK (white_points = 0 AND black_points = 0)")
                .execute(tc).await?;
            let result = execute_at(game.id, Command::Control {
                user_id: game.white_id,
                control: GameControl::Resign(Color::White),
            }, game.created_at, tc).await;
            assert!(matches!(result, Err(DbError::InternalError { ref reason }) if reason.contains("reject_test_award")));
            let unchanged = Game::find_by_uuid(&game.id, tc).await?;
            assert!(!unchanged.finished);
            assert_eq!(unchanged.white_rating_change, game.white_rating_change);
            assert_eq!(unchanged.black_rating_change, game.black_rating_change);
            assert_eq!(arena_game_results::table.count().get_result::<i64>(tc).await?, 0);
            Err(DbError::SerializationConflict)
        }).await;
        assert!(matches!(rollback, Err(DbError::SerializationConflict)));

        let terminal = finish_by_resignation(&game, game.created_at, &mut conn).await;
        let (white, black, white_doubled, black_doubled) = arena_game_results::table
            .find(game.id)
            .select((
                arena_game_results::white_points,
                arena_game_results::black_points,
                arena_game_results::white_doubled,
                arena_game_results::black_doubled,
            ))
            .first::<(i64, i64, bool, bool)>(&mut conn)
            .await
            .expect("persist the engine award");
        assert_eq!(
            (white, black, white_doubled, black_doubled),
            (0, 2, false, false)
        );
        let retried = execute_at(game.id, Command::SettleDeadline, game.created_at, &mut conn)
            .await
            .expect("retry the terminal command");
        assert!(matches!(
            retried,
            Outcome::Applied {
                newly_terminal: false,
                ..
            }
        ));
        assert_eq!(
            arena_game_results::table
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            1
        );
        let state = load_arena_state_for_read(tournament.clone(), &mut conn)
            .await
            .unwrap();
        let persisted = state
            .games
            .iter()
            .find(|entry| entry.game.id == terminal.id)
            .unwrap();
        assert_eq!(persisted.award.unwrap().awarded_points[1].value(), 2);
        let winner = game.black_id;
        for win in 2..=3 {
            let next = arena::pair_waiting_with_presence(
                tournament.id,
                move |_| HashSet::from([game.white_id, game.black_id]),
                &mut conn,
            )
            .await
            .unwrap()
            .new_games
            .remove(0);
            let loser_color = next.user_color(winner).unwrap().opposite_color();
            execute_at(
                next.id,
                Command::Control {
                    user_id: if loser_color == Color::White {
                        next.white_id
                    } else {
                        next.black_id
                    },
                    control: GameControl::Resign(loser_color),
                },
                next.created_at,
                &mut conn,
            )
            .await
            .unwrap();
            let (white, black, white_doubled, black_doubled) = arena_game_results::table
                .find(next.id)
                .select((
                    arena_game_results::white_points,
                    arena_game_results::black_points,
                    arena_game_results::white_doubled,
                    arena_game_results::black_doubled,
                ))
                .first::<(i64, i64, bool, bool)>(&mut conn)
                .await
                .unwrap();
            let points = if next.white_id == winner {
                white
            } else {
                black
            };
            let doubled = if next.white_id == winner {
                white_doubled
            } else {
                black_doubled
            };
            assert_eq!(points, if win == 3 { 4 } else { 2 });
            assert_eq!(doubled, win == 3);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disjoint_completions_commit_independently_and_finalization_keeps_only_visible_awards()
    {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let (tournament, _, games) = start_arena_fixture("award_disjoint", 4, &mut conn).await;
        let first = games[0].clone();
        let second = games[1].clone();
        let cutoff = second.created_at + Duration::microseconds(1);
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(cutoff - Duration::seconds(3_600))))
            .execute(&mut conn)
            .await
            .unwrap();
        let pool = db.pool.clone();
        conn.transaction::<_, DbError, _>(async |tc| {
            finish_by_resignation(&first, first.created_at, tc).await;
            let work = tokio::spawn(async move {
                let mut other = get_conn(&pool).await.unwrap();
                finish_by_resignation(&second, second.created_at, &mut other).await;
                let finalized = arena::finalize_due(tournament.id, &mut other)
                    .await
                    .unwrap();
                assert!(finalized.finished_now);
                assert_eq!(
                    included_arena_game_ids(tournament.id, &mut other)
                        .await
                        .unwrap(),
                    vec![second.id]
                );
            });
            tokio::time::timeout(std::time::Duration::from_secs(5), work)
                .await
                .expect("disjoint completion and finalization do not wait for the first completion")
                .expect("concurrent commands succeed");
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(
            arena_game_results::table
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            2
        );
        let snapshot = load_by_id(tournament.id, &mut conn).await.unwrap();
        let TournamentFormatResponse::Arena {
            games: included, ..
        } = snapshot.format
        else {
            unreachable!()
        };
        assert_eq!(included.len(), 1);
        assert_eq!(included[0].game.game_id, GameId(games[1].nanoid.clone()));
        assert!(
            Game::find_by_uuid(&first.id, &mut conn)
                .await
                .unwrap()
                .finished
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn imported_arena_finalization_rolls_back_with_its_owning_transaction() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let (tournament, _, games) = start_arena_fixture("award_nested", 2, &mut conn).await;
        let game = games.into_iter().next().unwrap();
        assert!(arena::finalize_due_in_transaction(tournament.id, &mut conn)
            .await
            .is_err());
        let rollback = conn
            .transaction::<(), DbError, _>(async |tc| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .execute(tc)
                    .await?;
                finish_by_resignation(&game, game.created_at, tc).await;
                update(tournaments::table.find(tournament.id))
                    .set(tournaments::starts_at.eq(Some(
                        game.created_at + Duration::microseconds(1) - Duration::seconds(3_600),
                    )))
                    .execute(tc)
                    .await?;
                assert!(arena::finalize_due(tournament.id, tc).await.is_err());
                assert!(TournamentFinalOutcome::load(tournament.id, tc)
                    .await?
                    .is_none());
                let finalized = arena::finalize_due_in_transaction(tournament.id, tc).await?;
                assert!(finalized.finished_now);
                assert_eq!(
                    included_arena_game_ids(tournament.id, tc).await?,
                    vec![game.id]
                );
                Err(DbError::SerializationConflict)
            })
            .await;
        assert!(matches!(rollback, Err(DbError::SerializationConflict)));
        assert!(
            !Game::find_by_uuid(&game.id, &mut conn)
                .await
                .unwrap()
                .finished
        );
        assert_eq!(
            Tournament::find(tournament.id, &mut conn)
                .await
                .unwrap()
                .status(),
            TournamentStatus::InProgress
        );
        assert!(TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            arena_game_results::table
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            0
        );
    }

    #[derive(QueryableByName)]
    struct BackendPid {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        pid: i32,
    }

    #[derive(QueryableByName)]
    struct BackendWait {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        blocked: bool,
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finalizer_includes_join_committed_while_waiting_for_tournament_lock() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let (tournament, players, _) = start_arena_fixture("final_join", 2, &mut conn).await;
        let entrant = create_user("final_join_late", &mut conn).await.id;
        let cutoff = tournament.starts_at.unwrap() + Duration::seconds(3_600);
        let mut finalizer_conn = db.pool.get_owned().await.unwrap();
        let BackendPid { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
            .get_result(&mut finalizer_conn)
            .await
            .unwrap();
        let mut observer = get_conn(&db.pool).await.unwrap();

        let finalizer = conn
            .transaction::<_, DbError, _>(async |tc| {
                // The join passes its cutoff, but its membership is still uncommitted
                // when the finalizer reaches the scoring cutoff on its own clock.
                let joined =
                    arena::join_with_presence(tournament.id, entrant, |_| HashSet::new(), tc)
                        .await?;
                assert!(joined.joined_now);
                let finalizer = tokio::spawn(async move {
                    finalize_due_with_clock(tournament.id, move || cutoff, &mut finalizer_conn)
                        .await
                });
                wait_for_backend_lock(pid, &mut observer).await;
                Ok(finalizer)
            })
            .await
            .unwrap();

        let finalized = tokio::time::timeout(std::time::Duration::from_secs(5), finalizer)
            .await
            .unwrap()
            .unwrap()
            .expect("finalize after the accepted join commits");
        assert!(finalized.finished_now);
        let frozen = TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap()
            .unwrap();
        let expected = players.into_iter().chain([entrant]).collect::<HashSet<_>>();
        assert_eq!(
            frozen
                .arena_ratings
                .unwrap()
                .into_iter()
                .map(|rating| rating.user_id)
                .collect::<HashSet<_>>(),
            expected,
        );
        let snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("finished projection includes every persisted participant");
        assert_eq!(
            snapshot
                .standings
                .unwrap()
                .groups
                .iter()
                .flat_map(|group| &group.rows)
                .map(|row| row.user_id)
                .collect::<HashSet<_>>(),
            expected,
        );
    }

    async fn wait_for_backend_lock(pid: i32, observer: &mut DbConn<'_>) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let BackendWait { blocked } =
                    diesel::sql_query("SELECT cardinality(pg_blocking_pids($1)) > 0 AS blocked")
                        .bind::<diesel::sql_types::Integer, _>(pid)
                        .get_result(observer)
                        .await
                        .unwrap();
                if blocked {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("finalizer reaches the expected database lock");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finalizer_captures_completion_and_ratings_from_the_same_snapshot() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let (tournament, _, games) = start_arena_fixture("award_snapshot", 2, &mut conn).await;
        let game = games.into_iter().next().unwrap();
        let cutoff = game.created_at + Duration::microseconds(1);
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(cutoff - Duration::seconds(3_600))))
            .execute(&mut conn)
            .await
            .unwrap();
        let before = load_by_id(tournament.id, &mut conn).await.unwrap();
        let memberships_before = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap();
        let mut finalizer_conn = db.pool.get_owned().await.unwrap();
        let BackendPid { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
            .get_result(&mut finalizer_conn)
            .await
            .unwrap();
        let mut observer = get_conn(&db.pool).await.unwrap();
        let finalizer = conn
            .transaction::<_, DbError, _>(async |tc| {
                // Block access to awards while a completion commits. Whether the
                // snapshot starts before or after that commit, its ratings and
                // included result must describe the same side of the commit.
                diesel::sql_query("LOCK TABLE arena_game_results IN ACCESS EXCLUSIVE MODE")
                    .execute(tc)
                    .await?;
                let finalizer = tokio::spawn(async move {
                    arena::finalize_due(tournament.id, &mut finalizer_conn).await
                });
                wait_for_backend_lock(pid, &mut observer).await;
                finish_by_resignation(&game, game.created_at, tc).await;
                Ok(finalizer)
            })
            .await
            .unwrap();
        let finalized = tokio::time::timeout(std::time::Duration::from_secs(5), finalizer)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(finalized.finished_now);
        assert!(
            Game::find_by_uuid(&game.id, &mut conn)
                .await
                .unwrap()
                .finished
        );
        assert_eq!(
            arena_game_results::table
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            1
        );
        let included = included_arena_game_ids(tournament.id, &mut conn)
            .await
            .unwrap();
        let memberships_after = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap();
        assert!(memberships_before
            .iter()
            .zip(&memberships_after)
            .any(|(before, after)| before.arena_rating != after.arena_rating));
        let expected_memberships = if included.is_empty() {
            memberships_before
        } else {
            assert_eq!(included, vec![game.id]);
            memberships_after
        };
        let frozen = TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            frozen
                .arena_ratings
                .unwrap()
                .into_iter()
                .map(|rating| (rating.user_id, rating.rating))
                .collect::<HashMap<_, _>>(),
            expected_memberships
                .into_iter()
                .map(|membership| (membership.user_id, membership.arena_rating.unwrap()))
                .collect::<HashMap<_, _>>(),
            "completion awards and frozen ratings must be captured atomically",
        );
        let after = load_by_id(tournament.id, &mut conn).await.unwrap();
        if included.is_empty() {
            assert_eq!(after.standings, before.standings);
        }
        let TournamentFormatResponse::Arena {
            games,
            player_stats,
            ..
        } = after.format
        else {
            panic!("Arena format")
        };
        assert_eq!(games.len(), included.len());
        assert!(player_stats
            .iter()
            .all(|stats| stats.games_scored as usize == included.len()));
    }

    async fn included_arena_game_ids(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        Ok(tournament_final_arena_results::table
            .filter(tournament_final_arena_results::tournament_id.eq(tournament_id))
            .select(tournament_final_arena_results::game_id)
            .order(tournament_final_arena_results::game_id)
            .load(conn)
            .await?)
    }

    async fn start_arena_fixture(
        prefix: &str,
        player_count: usize,
        conn: &mut DbConn<'_>,
    ) -> (Tournament, Vec<Uuid>, Vec<Game>) {
        let organizer = create_user(&format!("{prefix}_org"), conn).await;
        let mut players = Vec::with_capacity(player_count);
        for index in 0..player_count {
            players.push(create_user(&format!("{prefix}_{index}"), conn).await.id);
        }
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            prefix,
            TournamentStatus::NotStarted,
            Some(Utc::now() + Duration::hours(1)),
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Arena(ArenaConfig {
                    duration_seconds: NonZeroU32::new(3_600).unwrap(),
                    game_clock: RealtimeClock {
                        base_seconds: NonZeroU32::new(180).unwrap(),
                        increment_seconds: 1,
                    },
                }),
            },
            conn,
        )
        .await;
        update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::seconds(1))))
            .execute(conn)
            .await
            .expect("make Arena tournament due");
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), conn).await;
        let present = players.clone();
        let started = arena::start_scheduled_with_presence(
            tournament.id,
            move |_| present.iter().copied().collect(),
            conn,
        )
        .await
        .expect("start Arena fixture");
        (started.tournament, players, started.new_games)
    }

    async fn finish_by_resignation(
        game: &Game,
        finished_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Game {
        let outcome = execute_at(
            game.id,
            Command::Control {
                user_id: game.white_id,
                control: GameControl::Resign(Color::White),
            },
            finished_at,
            conn,
        )
        .await
        .expect("finish Arena Game by resignation");
        let Outcome::Applied { game, .. } = outcome else {
            panic!("Arena resignation updates the Game")
        };
        game
    }

    async fn freeze_arena_at(
        tournament_id: Uuid,
        cutoff: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> TournamentFinalOutcome {
        update(tournaments::table.find(tournament_id))
            .set(tournaments::starts_at.eq(Some(cutoff - Duration::seconds(3_600))))
            .execute(conn)
            .await
            .expect("move Arena cutoff to the requested instant");
        finalize_due_with_clock(tournament_id, move || cutoff, conn)
            .await
            .expect("freeze Arena outcome");
        TournamentFinalOutcome::load(tournament_id, conn)
            .await
            .expect("load frozen Arena outcome")
            .expect("Arena finalization persisted its outcome")
    }
}
