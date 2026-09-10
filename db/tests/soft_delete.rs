mod common;

use chrono::{DateTime, Duration, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    get_conn,
    helpers::run_serializable,
    models::{Game, NewGame, NewTournament, NewUser, Tournament, TournamentSlot, User},
    schema::{games, tournaments, tournaments_organizers, tournaments_users, users},
    tournaments::{arena, fixed_field},
    DbConn,
};
use diesel::{prelude::*, result::Error as DieselError};
use diesel_async::RunQueryDsl;
use hive_lib::{Color, Direction, GameControl, GameResult, GameStatus, GameType, Position, Turn};
use shared_types::{
    tournament::{
        arena::{Config as ArenaConfig, MIN_DURATION_SECONDS as ARENA_MIN_DURATION_SECONDS},
        elimination::{
            ClinchPolicy,
            Config as EliminationConfig,
            EntrantSide,
            SeriesPhase,
            SeriesPlan,
            SetLimit,
            Topology,
        },
        round_robin::Config as RoundRobinConfig,
        swiss::{Config as SwissConfig, PrimaryScore as DoubleSwissPrimaryScore},
        BotAdmission,
        Clock,
        Config,
        FormatConfig,
        RealtimeClock,
    },
    Conclusion,
    GameSpeed,
    GameStart,
    TimeMode,
    TournamentDetails,
    TournamentGameResult,
    TournamentId,
    TournamentStatus,
};
use std::{collections::HashSet, num::NonZeroU32, sync::Arc};
use tokio::sync::Barrier;
use uuid::Uuid;

const DELETED_USERNAME_PREFIX: &str = "deleted_user_";

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_anonymizes_user_and_rejects_tombstone_login() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("delete_me", &mut conn).await;
    diesel::update(users::table.find(user.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .expect("mark user as admin");

    let report = user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    assert!(report.deleted_games.is_empty());
    assert!(report.resigned_games.is_empty());

    let deleted = User::find_by_uuid(&user.id, &mut conn)
        .await
        .expect("load historical deleted user");
    assert!(deleted.deleted);
    assert_eq!(
        deleted.username,
        format!("{DELETED_USERNAME_PREFIX}{}", user.id)
    );
    assert_eq!(deleted.normalized_username, deleted.username);
    assert_eq!(
        deleted.email,
        format!("{}@deleted.invalid", deleted.username)
    );
    assert_eq!(deleted.password, "replacement-password-hash");
    assert!(!deleted.admin);

    assert!(!User::username_exists("delete_me", &mut conn)
        .await
        .expect("old username is available"));
    assert!(!User::username_exists("deleted_user_999", &mut conn)
        .await
        .expect("unknown tombstone username is absent"));
    assert!(matches!(
        NewUser::new(
            "deleted_user_999",
            "new-password",
            "deleted-user-999@example.com"
        )
        .expect_err("deleted username pattern is reserved"),
        DbError::InvalidInput { .. }
    ));
    assert!(matches!(
        User::find_for_login(&deleted.username, &mut conn)
            .await
            .expect_err("deleted user cannot log in"),
        DbError::NotFound { .. }
    ));
    assert!(matches!(
        User::find_for_login(&deleted.email, &mut conn)
            .await
            .expect_err("deleted user cannot log in by tombstone email"),
        DbError::NotFound { .. }
    ));

    let replacement = NewUser::new("delete_me", "new-password", "delete_me@example.com")
        .expect("old identity can be reused");
    User::create(replacement, &mut conn)
        .await
        .expect("create replacement account");
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_aborts_early_games_and_resigns_started_games() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let deleting_user = create_user("soon_gone", &mut conn).await;
    let opponent = create_user("opponent", &mut conn).await;

    let early_game = create_abortable_game(deleting_user.id, opponent.id, &mut conn).await;
    let started_game = create_abortable_game(deleting_user.id, opponent.id, &mut conn).await;
    let resumed_game = create_abortable_game(deleting_user.id, opponent.id, &mut conn).await;
    for game_id in [started_game.id, resumed_game.id] {
        execute(
            game_id,
            Command::Move {
                user_id: deleting_user.id,
                turn: Turn::Move(
                    "wA1".parse().expect("parse white ant"),
                    Position::initial_spawn_position(),
                ),
                compensation: 0.0,
            },
            &mut conn,
        )
        .await
        .expect("play the white opening move");
        execute(
            game_id,
            Command::Move {
                user_id: opponent.id,
                turn: Turn::Move(
                    "bA1".parse().expect("parse black ant"),
                    Position::initial_spawn_position().to(Direction::E),
                ),
                compensation: 0.0,
            },
            &mut conn,
        )
        .await
        .expect("play the black opening move");
    }
    for (actor, control) in [
        (deleting_user.id, GameControl::TakebackRequest(Color::White)),
        (opponent.id, GameControl::TakebackAccept(Color::Black)),
        (deleting_user.id, GameControl::TakebackRequest(Color::White)),
        (opponent.id, GameControl::TakebackAccept(Color::Black)),
    ] {
        execute(
            resumed_game.id,
            Command::Control {
                user_id: actor,
                control,
            },
            &mut conn,
        )
        .await
        .expect("take the started game back to turn zero");
    }
    let resumed_game = Game::find_by_uuid(&resumed_game.id, &mut conn)
        .await
        .expect("reload resumed opening");
    assert_eq!(resumed_game.turn, 0);
    assert_eq!(resumed_game.game_status, GameStatus::InProgress.to_string());

    let report = deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    assert_eq!(report.deleted_games.len(), 1);
    assert_eq!(report.deleted_games[0].nanoid, early_game.nanoid);
    assert_eq!(report.resigned_games.len(), 2);
    assert_eq!(
        report
            .resigned_games
            .iter()
            .map(|game| game.id)
            .collect::<HashSet<_>>(),
        HashSet::from([started_game.id, resumed_game.id])
    );

    let early_exists = games::table
        .find(early_game.id)
        .select(games::id)
        .first::<Uuid>(&mut conn)
        .await;
    assert!(matches!(early_exists, Err(DieselError::NotFound)));

    for game_id in [started_game.id, resumed_game.id] {
        let resigned = Game::find_by_uuid(&game_id, &mut conn)
            .await
            .expect("load resigned game");
        assert!(resigned.finished);
        assert_eq!(resigned.conclusion, Conclusion::Resigned.to_string());
        assert_eq!(
            resigned.game_status,
            GameStatus::Finished(GameResult::Winner(Color::Black)).to_string()
        );
        assert!(resigned.white_rating.is_some());
        assert!(resigned.black_rating.is_some());
        assert!(resigned.white_rating_change.is_some());
        assert!(resigned.black_rating_change.is_some());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_deletes_unstarted_tournaments_organized_by_user_but_keeps_started() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let deleting_organizer = create_user("deleted_organizer", &mut conn).await;
    let player = create_user("organized_player", &mut conn).await;

    let unstarted = create_double_swiss_tournament(deleting_organizer.id, &mut conn).await;
    unstarted
        .join(&player.id, &mut conn)
        .await
        .expect("join unstarted tournament");
    let started = create_realtime_tournament(deleting_organizer.id, &mut conn).await;
    let surviving_organizer = create_user("remaining_organizer", &mut conn).await;
    let shared = common::tournament::create_rr_tournament(
        deleting_organizer.id,
        "Shared organizers",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let shared_id = TournamentId(shared.nanoid.clone());
    run_serializable(&mut conn, |tc| {
        let id = shared_id.clone();
        Box::pin(async move {
            let tournament = Tournament::find_by_tournament_id_for_update(&id, tc).await?;
            tournament
                .invite_organizer(deleting_organizer.id, surviving_organizer.id, tc)
                .await?;
            tournament
                .accept_organizer_invitation(surviving_organizer.id, tc)
                .await
        })
    })
    .await
    .expect("accept additional organizer");

    deleting_organizer
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete organizer");

    let deleted_tournament = tournaments::table
        .find(unstarted.id)
        .select(tournaments::id)
        .first::<Uuid>(&mut conn)
        .await;
    assert!(matches!(deleted_tournament, Err(DieselError::NotFound)));

    let deleted_player_row = tournaments_users::table
        .find((unstarted.id, player.id))
        .select(tournaments_users::user_id)
        .first::<Uuid>(&mut conn)
        .await;
    assert!(matches!(deleted_player_row, Err(DieselError::NotFound)));

    let deleted_organizer_row = tournaments_organizers::table
        .find((unstarted.id, deleting_organizer.id))
        .select(tournaments_organizers::organizer_id)
        .first::<Uuid>(&mut conn)
        .await;
    assert!(matches!(deleted_organizer_row, Err(DieselError::NotFound)));

    let kept_shared = Tournament::find(shared.id, &mut conn)
        .await
        .expect("co-organizer preserves unstarted tournament");
    assert_eq!(
        kept_shared
            .organizers(&mut conn)
            .await
            .unwrap()
            .iter()
            .map(|user| user.id)
            .collect::<Vec<_>>(),
        vec![surviving_organizer.id]
    );

    let kept_started = Tournament::find(started.id, &mut conn)
        .await
        .expect("started tournament remains");
    assert_eq!(kept_started.status(), TournamentStatus::InProgress);
    tournaments_organizers::table
        .find((started.id, deleting_organizer.id))
        .select(tournaments_organizers::organizer_id)
        .first::<Uuid>(&mut conn)
        .await
        .expect("started tournament organizer remains for admin handling");
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_withdraws_an_eliminated_entrant_without_exposing_a_user_action() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("elim_delete_org", &mut conn).await;
    let deleting_user = create_user("elim_delete_player", &mut conn).await;
    let opponents = [
        create_user("elim_delete_opp_0", &mut conn).await,
        create_user("elim_delete_opp_1", &mut conn).await,
        create_user("elim_delete_opp_2", &mut conn).await,
    ];
    let players = [
        deleting_user.id,
        opponents[0].id,
        opponents[1].id,
        opponents[2].id,
    ];
    let tournament = Tournament::create(
        organizer.id,
        &NewTournament::new(TournamentDetails {
            name: String::from("Soft delete eliminated entrant"),
            description: Some(String::from(
                "A deterministic account-deletion elimination fixture.",
            )),
            seats: Some(4),
            min_seats: 4,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Elimination(EliminationConfig {
                    topology: Topology::Single { bronze: false },
                    default_plan: SeriesPlan {
                        phases: vec![SeriesPhase {
                            games_per_set: 1,
                            color_order: vec![EntrantSide::First],
                            set_limit: SetLimit::UntilDecisive,
                            clinch: ClinchPolicy::PlayAll,
                            clock: realtime_clock(),
                        }],
                    },
                    stage_overrides: Vec::new(),
                }),
            },
        })
        .expect("build elimination tournament"),
        &mut conn,
    )
    .await
    .expect("insert elimination tournament");
    common::tournament::insert_unfrozen_memberships(
        tournament.id,
        &players,
        common::tournament::fixed_instant(),
        &mut conn,
    )
    .await;
    let prepared = fixed_field::prepare_elimination_start(
        tournament.id,
        organizer.id,
        players.to_vec(),
        &mut conn,
    )
    .await
    .expect("prepare elimination tournament");
    let setup = prepared.start_setup().expect("prepared setup");
    let opening_wave = fixed_field::confirm_elimination_start(
        tournament.id,
        organizer.id,
        setup.id,
        setup.seeded_players,
        &mut conn,
    )
    .await
    .expect("start elimination tournament")
    .games;
    let deleting_game = opening_wave
        .iter()
        .find(|game| game.user_is_player(deleting_user.id))
        .expect("deleting entrant has an opening match");
    let opponent_wins = if deleting_game.white_id == deleting_user.id {
        TournamentGameResult::Winner(Color::Black)
    } else {
        TournamentGameResult::Winner(Color::White)
    };
    fixed_field::adjudicate_slot_atomic(
        tournament.id,
        deleting_game
            .tournament_slot_id
            .expect("elimination game owns a Slot"),
        opponent_wins,
        None,
        organizer.id,
        &mut conn,
    )
    .await
    .expect("eliminate the account-deletion entrant");

    let user_withdrawal =
        fixed_field::withdraw_player(tournament.id, deleting_user.id, deleting_user.id, &mut conn)
            .await;
    assert!(matches!(
        user_withdrawal,
        Err(DbError::InvalidAction { .. })
    ));

    let report = deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete an eliminated tournament entrant");
    assert_eq!(
        report.withdrawn_tournament_ids,
        vec![TournamentId(tournament.nanoid.clone())]
    );
    assert!(tournaments_users::table
        .find((tournament.id, deleting_user.id))
        .select(tournaments_users::withdrawn_at)
        .first::<Option<DateTime<Utc>>>(&mut conn)
        .await
        .expect("load deleted entrant membership")
        .is_some());
    assert!(
        User::find_by_uuid(&deleting_user.id, &mut conn)
            .await
            .expect("reload deleted entrant")
            .deleted
    );
    assert_eq!(
        Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload unfinished elimination tournament")
            .status(),
        TournamentStatus::InProgress
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_is_atomic_across_formats_and_never_releases_the_deleted_player() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("mix_delete_org", &mut conn).await;
    let deleting_user = create_user("mix_delete_player", &mut conn).await;
    let opponent = create_user("mix_delete_opponent", &mut conn).await;
    let swiss_opponent_b = create_user("mix_delete_swiss_b", &mut conn).await;
    let swiss_opponent_c = create_user("mix_delete_swiss_c", &mut conn).await;
    let swiss_opponent_d = create_user("mix_delete_swiss_d", &mut conn).await;
    let swiss_opponent_e = create_user("mix_delete_swiss_e", &mut conn).await;
    let (round_robin, games) = create_started_round_robin(
        "Soft delete rollback Round Robin",
        organizer.id,
        [deleting_user.id, opponent.id],
        &mut conn,
    )
    .await;
    let swiss_tournament = create_started_swiss_tournament(
        organizer.id,
        [
            deleting_user.id,
            opponent.id,
            swiss_opponent_b.id,
            swiss_opponent_c.id,
            swiss_opponent_d.id,
            swiss_opponent_e.id,
        ],
        &mut conn,
    )
    .await;
    let first_swiss_games = swiss_tournament
        .games(&mut conn)
        .await
        .expect("load the first Swiss round games");
    let original_swiss_game_ids = first_swiss_games
        .iter()
        .map(|game| game.id)
        .collect::<HashSet<_>>();
    let deleting_swiss_game = first_swiss_games
        .iter()
        .find(|game| game.user_is_player(deleting_user.id))
        .expect("the deleting player has a Swiss pairing")
        .clone();
    let independent_swiss_game = first_swiss_games
        .iter()
        .find(|game| !game.user_is_player(deleting_user.id))
        .expect("the Swiss round has an independent game");
    fixed_field::adjudicate_slot_atomic(
        swiss_tournament.id,
        independent_swiss_game
            .tournament_slot_id
            .expect("Swiss game owns a slot"),
        TournamentGameResult::Winner(Color::White),
        None,
        organizer.id,
        &mut conn,
    )
    .await
    .expect("settle the independent Swiss game");
    let arena = create_arena_tournament(organizer.id, &mut conn).await;
    arena
        .join(&deleting_user.id, &mut conn)
        .await
        .expect("join Arena before account deletion");
    arena
        .join(&opponent.id, &mut conn)
        .await
        .expect("join Arena opponent");
    let arena_starts_at =
        DateTime::from_timestamp_micros((Utc::now() - Duration::seconds(1)).timestamp_micros())
            .expect("Arena start timestamp");
    diesel::update(tournaments::table.find(arena.id))
        .set(tournaments::starts_at.eq(Some(arena_starts_at)))
        .execute(&mut conn)
        .await
        .expect("move Arena fixture to its start boundary");
    diesel::update(tournaments_users::table.filter(tournaments_users::tournament_id.eq(arena.id)))
        .set(tournaments_users::accepted_at.eq(arena_starts_at - Duration::seconds(1)))
        .execute(&mut conn)
        .await
        .expect("move Arena fixture memberships before its start boundary");
    let deleting_id = deleting_user.id;
    let opponent_id = opponent.id;
    let arena_started = arena::start_scheduled_with_presence(
        arena.id,
        move |_: &[Uuid]| HashSet::from([deleting_id, opponent_id]),
        &mut conn,
    )
    .await
    .expect("start Arena before account deletion");
    assert_eq!(arena_started.new_games.len(), 1);

    let collision = create_user("mix_delete_collision", &mut conn).await;
    let deleted_username = format!("{DELETED_USERNAME_PREFIX}{}", deleting_user.id);
    diesel::update(users::table.find(collision.id))
        .set((
            users::username.eq(&deleted_username),
            users::normalized_username.eq(&deleted_username),
        ))
        .execute(&mut conn)
        .await
        .expect("reserve the deletion tombstone identity");
    assert!(deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .is_err());
    assert!(
        !User::find_by_uuid(&deleting_user.id, &mut conn)
            .await
            .expect("reload account after rolled-back deletion")
            .deleted
    );
    for game_id in [
        games[0].id,
        deleting_swiss_game.id,
        arena_started.new_games[0].id,
    ] {
        assert!(
            !Game::find_by_uuid(&game_id, &mut conn)
                .await
                .expect("reload game after rolled-back deletion")
                .finished
        );
    }
    for (tournament_id, game) in [
        (round_robin.id, &games[0]),
        (swiss_tournament.id, &deleting_swiss_game),
    ] {
        assert!(TournamentSlot::find(
            tournament_id,
            game.tournament_slot_id
                .expect("fixed-field game owns a slot"),
            &mut conn,
        )
        .await
        .expect("reload slot after rolled-back deletion")
        .resolution
        .is_none());
    }
    diesel::update(users::table.find(collision.id))
        .set((
            users::username.eq("mix_delete_collision"),
            users::normalized_username.eq("mix_delete_collision"),
        ))
        .execute(&mut conn)
        .await
        .expect("release the deletion tombstone identity");

    let report = deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("delete the account and update every tournament atomically");

    assert!(
        User::find_by_uuid(&deleting_user.id, &mut conn)
            .await
            .expect("load deleted account")
            .deleted
    );
    assert!(report
        .tournament_terminal_games
        .iter()
        .all(|game| game.finished));

    for (tournament_id, game) in [
        (round_robin.id, &games[0]),
        (swiss_tournament.id, &deleting_swiss_game),
    ] {
        assert!(
            Game::find_by_uuid(&game.id, &mut conn)
                .await
                .expect("reload terminal fixed-field game")
                .finished
        );
        assert!(TournamentSlot::find(
            tournament_id,
            game.tournament_slot_id
                .expect("fixed-field game owns a slot"),
            &mut conn,
        )
        .await
        .expect("reload terminal fixed-field slot")
        .resolution
        .is_some());
    }
    assert!(
        Game::find_by_uuid(&arena_started.new_games[0].id, &mut conn)
            .await
            .expect("reload terminal Arena game")
            .finished
    );

    let released_game_ids = report
        .fixed_field_commits
        .iter()
        .flat_map(|effects| effects.released_game_ids.iter().cloned())
        .collect::<Vec<_>>();
    let released_games = Game::find_by_nanoids(&released_game_ids, &mut conn)
        .await
        .expect("load games released during account deletion");
    assert!(released_games
        .iter()
        .all(|game| !game.user_is_player(deleting_user.id)));
    let persisted_swiss_games = Tournament::find(swiss_tournament.id, &mut conn)
        .await
        .expect("reload Swiss tournament")
        .games(&mut conn)
        .await
        .expect("load Swiss games after account deletion");
    assert!(persisted_swiss_games
        .iter()
        .filter(|game| !original_swiss_game_ids.contains(&game.id))
        .all(|game| !game.user_is_player(deleting_user.id)));

    let deleting_id = deleting_user.id;
    let opponent_id = opponent.id;
    let post_delete_pairing = arena::pair_waiting_with_presence(
        arena.id,
        move |_: &[Uuid]| HashSet::from([deleting_id, opponent_id]),
        &mut conn,
    )
    .await
    .expect("pair Arena after account deletion");
    assert!(post_delete_pairing
        .new_games
        .iter()
        .all(|game| !game.user_is_player(deleting_user.id)));
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_folds_a_preexisting_terminal_marker_before_withdrawal() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("recover_del_org", &mut conn).await;
    let deleting_user = create_user("recover_del_player", &mut conn).await;
    let opponent = create_user("recover_del_opp", &mut conn).await;
    let (tournament, games) = create_started_round_robin(
        "Soft delete terminal recovery",
        organizer.id,
        [deleting_user.id, opponent.id],
        &mut conn,
    )
    .await;
    let color = games[0]
        .user_color(deleting_user.id)
        .expect("deleting user is a player");
    execute(
        games[0].id,
        Command::StartReady {
            user_id: deleting_user.id,
        },
        &mut conn,
    )
    .await
    .expect("begin the Ready tournament game");
    execute(
        games[0].id,
        Command::Move {
            user_id: games[0].white_id,
            turn: Turn::Move(
                "wA1".parse().expect("parse white ant"),
                Position::initial_spawn_position(),
            ),
            compensation: 0.0,
        },
        &mut conn,
    )
    .await
    .expect("play the white opening move");
    execute(
        games[0].id,
        Command::Move {
            user_id: games[0].black_id,
            turn: Turn::Move(
                "bA1".parse().expect("parse black ant"),
                Position::initial_spawn_position().to(Direction::E),
            ),
            compensation: 0.0,
        },
        &mut conn,
    )
    .await
    .expect("play the black opening move");
    let mutation = execute(
        games[0].id,
        Command::Control {
            user_id: deleting_user.id,
            control: GameControl::Resign(color),
        },
        &mut conn,
    )
    .await
    .expect("leave a terminal tournament game for recovery");
    let Outcome::Applied {
        game,
        newly_terminal: true,
        ..
    } = mutation
    else {
        panic!("resignation commits a terminal fixed-field Game")
    };
    assert!(game.finished);
    let slot_id = game
        .tournament_slot_id
        .expect("terminal fixed-field Game owns a Slot");
    assert!(TournamentSlot::find(tournament.id, slot_id, &mut conn)
        .await
        .expect("load durable recovery marker")
        .resolution
        .is_none());

    let report = deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("recover the terminal result and withdraw atomically");

    assert!(report.withdrawn_tournament_ids.is_empty());
    assert!(report.tournament_terminal_games.is_empty());
    assert_eq!(report.fixed_field_commits.len(), 1);
    let commit = &report.fixed_field_commits[0];
    assert!(commit.finished_now);
    assert_eq!(commit.affected_slot_ids, vec![slot_id]);
    assert!(commit.standings_changed);
    assert!(!commit.format_changed);
    assert!(commit.catalog_changed);
    assert_eq!(
        TournamentSlot::find(tournament.id, slot_id, &mut conn)
            .await
            .expect("reload reconciled Slot")
            .resolved_at,
        game.finished_at,
    );
    assert_eq!(
        Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload reconciled tournament")
            .status(),
        TournamentStatus::Finished,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_soft_delete_and_first_join_leave_no_deleted_membership() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let organizer = create_user("del_join_org", &mut setup_conn).await;
    let joining_user = create_user("del_join_player", &mut setup_conn).await;
    let tournament = common::tournament::create_rr_tournament(
        organizer.id,
        "Concurrent delete and join",
        TournamentStatus::NotStarted,
        &mut setup_conn,
    )
    .await;
    drop(setup_conn);

    let barrier = Arc::new(Barrier::new(2));
    let join_pool = db.pool.clone();
    let join_barrier = Arc::clone(&barrier);
    let tournament_id = TournamentId(tournament.nanoid.clone());
    let joining_user_id = joining_user.id;
    let join = async move {
        let mut conn = get_conn(&join_pool).await.expect("get join connection");
        join_barrier.wait().await;
        run_serializable(&mut conn, move |tc| {
            let tournament_id = tournament_id.clone();
            Box::pin(async move {
                let tournament =
                    Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
                tournament.join(&joining_user_id, tc).await
            })
        })
        .await
    };

    let delete_pool = db.pool.clone();
    let delete_barrier = Arc::clone(&barrier);
    let delete = async move {
        let mut conn = get_conn(&delete_pool)
            .await
            .expect("get account deletion connection");
        delete_barrier.wait().await;
        joining_user
            .soft_delete("replacement-password-hash", &mut conn)
            .await
    };

    let (_join_result, delete_result) = tokio::join!(join, delete);
    delete_result.expect("soft deletion eventually commits");

    let mut conn = get_conn(&db.pool)
        .await
        .expect("get verification connection");
    let deleted = User::find_by_uuid(&joining_user_id, &mut conn)
        .await
        .expect("reload deleted user");
    assert!(deleted.deleted);
    assert_eq!(
        tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(tournament.id))
            .filter(tournaments_users::user_id.eq(joining_user_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count residual membership"),
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn tournament_relationships_reject_a_soft_deleted_account() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("active_tourn_org", &mut conn).await;
    let deleted_user = create_user("deleted_tourn_player", &mut conn).await;
    let tournament = common::tournament::create_rr_tournament(
        organizer.id,
        "Deleted relationship guard",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    deleted_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete candidate player");

    assert!(matches!(
        tournament.join(&deleted_user.id, &mut conn).await,
        Err(DbError::NotFound { .. })
    ));
    assert!(matches!(
        tournament
            .create_invitation(&organizer.id, &deleted_user.id, &mut conn)
            .await,
        Err(DbError::NotFound { .. })
    ));
}

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn create_abortable_game(white_id: Uuid, black_id: Uuid, conn: &mut DbConn<'_>) -> Game {
    let now = Utc::now();
    let time_left = Some(60 * 1_000_000_000_i64);
    Game::create(
        NewGame {
            tournament_id: None,
            nanoid: nanoid::nanoid!(12),
            current_player_id: white_id,
            black_id,
            finished: false,
            game_status: GameStatus::NotStarted.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn: 0,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: None,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at: None,
            tournament_slot_id: None,
            arena_ordinal: None,
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: None,
        },
        conn,
    )
    .await
    .expect("insert game")
}

async fn create_started_round_robin(
    name: &str,
    organizer_id: Uuid,
    players: [Uuid; 2],
    conn: &mut DbConn<'_>,
) -> (Tournament, Vec<Game>) {
    let tournament = common::tournament::create_rr_tournament(
        organizer_id,
        name,
        TournamentStatus::NotStarted,
        conn,
    )
    .await;
    common::tournament::insert_unfrozen_memberships(
        tournament.id,
        &players,
        common::tournament::fixed_instant(),
        conn,
    )
    .await;
    let started =
        fixed_field::start_by_organizer(tournament.id, organizer_id, players.to_vec(), conn)
            .await
            .expect("start Round Robin fixture");
    (started.tournament, started.games)
}

async fn create_double_swiss_tournament(organizer_id: Uuid, conn: &mut DbConn<'_>) -> Tournament {
    let new_tournament = NewTournament::new(TournamentDetails {
        name: "Soft delete Double Swiss tournament".to_string(),
        description: Some(String::from(
            "A deterministic tournament fixture used for account deletion integration tests.",
        )),
        seats: Some(5),
        min_seats: 5,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: None,
        configuration: Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                0,
                realtime_clock(),
                DoubleSwissPrimaryScore::GamePoints,
            )),
        },
    })
    .expect("build Double Swiss tournament");
    Tournament::create(organizer_id, &new_tournament, conn)
        .await
        .expect("insert Double Swiss tournament")
}

async fn create_started_swiss_tournament(
    organizer_id: Uuid,
    players: [Uuid; 6],
    conn: &mut DbConn<'_>,
) -> Tournament {
    create_started_swiss(
        "Soft delete Swiss tournament",
        organizer_id,
        &players,
        SwissConfig::automatic_swiss(0, realtime_clock()),
        conn,
    )
    .await
}

async fn create_started_swiss(
    name: &str,
    organizer_id: Uuid,
    players: &[Uuid],
    configuration: SwissConfig,
    conn: &mut DbConn<'_>,
) -> Tournament {
    let new_tournament = NewTournament::new(TournamentDetails {
        name: name.to_string(),
        description: Some(String::from(
            "A deterministic tournament fixture used for account deletion integration tests.",
        )),
        seats: Some(i32::try_from(players.len()).expect("Swiss fixture player count fits i32")),
        min_seats: 5,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: None,
        configuration: Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(configuration),
        },
    })
    .expect("build Swiss tournament");
    let tournament = Tournament::create(organizer_id, &new_tournament, conn)
        .await
        .expect("insert Swiss tournament");
    common::tournament::insert_unfrozen_memberships(
        tournament.id,
        players,
        common::tournament::fixed_instant(),
        conn,
    )
    .await;
    fixed_field::start_by_organizer(tournament.id, organizer_id, players.to_vec(), conn)
        .await
        .expect("start Swiss tournament")
        .tournament
}

async fn create_arena_tournament(organizer_id: Uuid, conn: &mut DbConn<'_>) -> Tournament {
    let new_tournament = NewTournament::new(TournamentDetails {
        name: "Soft delete Arena tournament".to_string(),
        description: Some(String::from(
            "A deterministic tournament fixture used for account deletion integration tests.",
        )),
        seats: None,
        min_seats: 0,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: Some(Utc::now() + Duration::hours(1)),
        configuration: Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(ARENA_MIN_DURATION_SECONDS).unwrap(),
                match realtime_clock() {
                    Clock::Realtime(clock) => clock,
                    Clock::Correspondence(_) => unreachable!("fixture clock is realtime"),
                },
            )),
        },
    })
    .expect("build Arena tournament");
    Tournament::create(organizer_id, &new_tournament, conn)
        .await
        .expect("insert Arena tournament")
}

async fn create_realtime_tournament(organizer_id: Uuid, conn: &mut DbConn<'_>) -> Tournament {
    let new_tournament = NewTournament::new(TournamentDetails {
        name: "Soft delete tournament".to_string(),
        description: Some(String::from(
            "A deterministic tournament fixture used for account deletion integration tests.",
        )),
        seats: Some(2),
        min_seats: 2,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: None,
        configuration: Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
                NonZeroU32::new(1).unwrap(),
                realtime_clock(),
            )),
        },
    })
    .expect("build started tournament");
    let tournament = Tournament::create(organizer_id, &new_tournament, conn)
        .await
        .expect("insert tournament");
    common::tournament::apply_lifecycle(tournament, TournamentStatus::InProgress, conn).await
}

fn realtime_clock() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(60).unwrap(),
        increment_seconds: 0,
    })
}
