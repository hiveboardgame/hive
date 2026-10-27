use shared_types::tournament_view::{EliminationState, TournamentFormatResponse};
mod common;

use chrono::{Duration, Utc};
use common::{
    db::test_db,
    tournament::{create_user, realtime_clock},
};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{NewTournament, Tournament, TournamentUser, User},
    schema::{games, ratings, tournaments, users},
    tournaments::{fixed_field, public::load_by_id},
    DbConn,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{
    tournament::{
        elimination::{
            ClinchPolicy,
            Config as EliminationConfig,
            EntrantSide,
            SeriesPhase,
            SeriesPlan,
            SetLimit,
            Topology,
        },
        BotAdmission,
        Config,
        FormatConfig,
    },
    TournamentDetails,
    TournamentStatus,
};
use uuid::Uuid;

async fn fixture(conn: &mut DbConn<'_>) -> (Tournament, User, Vec<Uuid>) {
    let owner = create_user("setup_owner", conn).await;
    let mut players = Vec::new();
    for index in 0..3 {
        players.push(create_user(&format!("setup_player_{index}"), conn).await.id);
    }
    let tournament = Tournament::create(
        owner.id,
        &NewTournament::new(TournamentDetails {
            name: String::from("Bracket setup invariant fixture"),
            description: None,
            seats: Some(8),
            min_seats: 2,
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
        .expect("valid fixture"),
        conn,
    )
    .await
    .expect("create fixture");
    common::tournament::insert_unfrozen_memberships(tournament.id, &players, Utc::now(), conn)
        .await;
    (tournament, owner, players)
}

#[tokio::test(flavor = "multi_thread")]
async fn setup_is_exclusive_and_preserves_rounded_seeds_through_confirmation() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
    let mut ordered_players = players.clone();
    ordered_players.sort_unstable();
    for (player, rating) in ordered_players.iter().zip([1800.4, 1800.6, 1800.49]) {
        diesel::update(ratings::table.filter(ratings::user_uid.eq(player)))
            .set(ratings::rating.eq(rating))
            .execute(&mut conn)
            .await
            .unwrap();
    }
    let admin = create_user("setup_admin", &mut conn).await;
    diesel::update(users::table.find(admin.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();
    let prepared =
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players.clone(), &mut conn)
            .await
            .unwrap();
    let setup = prepared.start_setup().unwrap();
    assert_eq!(
        setup.seeded_players,
        vec![ordered_players[1], ordered_players[0], ordered_players[2]]
    );
    assert_eq!(setup.expires_at() - setup.opened_at, Duration::minutes(15));
    assert_eq!(prepared.status(), TournamentStatus::NotStarted);
    assert_eq!(
        games::table
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
    assert!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .iter()
            .all(|row| row.pairing_number.is_none())
    );
    assert!(matches!(
        fixed_field::prepare_elimination_start(tournament.id, admin.id, players.clone(), &mut conn)
            .await,
        Err(DbError::InvalidInput { .. })
    ));
    assert!(matches!(
        fixed_field::cancel_elimination_start(tournament.id, admin.id, setup.id, &mut conn).await,
        Err(DbError::InvalidInput { .. })
    ));
    assert!(matches!(
        fixed_field::confirm_elimination_start(
            tournament.id,
            admin.id,
            setup.id,
            setup.seeded_players.clone(),
            &mut conn
        )
        .await,
        Err(DbError::InvalidInput { .. })
    ));
    assert!(matches!(
        prepared.join(&admin.id, &mut conn).await,
        Err(DbError::InvalidAction { .. })
    ));
    assert!(matches!(
        prepared.leave(&players[0], &mut conn).await,
        Err(DbError::InvalidAction { .. })
    ));
    let mut invalid = setup.seeded_players.clone();
    invalid[1] = invalid[0];
    assert!(matches!(
        fixed_field::confirm_elimination_start(
            tournament.id,
            owner.id,
            setup.id,
            invalid,
            &mut conn
        )
        .await,
        Err(DbError::InvalidInput { .. })
    ));
    diesel::update(ratings::table.filter(ratings::user_uid.eq(setup.seeded_players[2])))
        .set(ratings::rating.eq(2900.0))
        .execute(&mut conn)
        .await
        .unwrap();
    let mut placement = setup.seeded_players.clone();
    placement.swap(0, 2);
    let started = fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        setup.id,
        placement.clone(),
        &mut conn,
    )
    .await
    .unwrap();
    assert_eq!(started.tournament.status(), TournamentStatus::InProgress);
    assert!(started.tournament.start_setup().is_none());
    assert_eq!(started.tournament.bracket_order(), Some(placement.clone()));
    assert!(started
        .games
        .iter()
        .all(|game| !game.user_is_player(placement[0])));
    let reloaded = Tournament::find(tournament.id, &mut conn).await.unwrap();
    let snapshot = load_by_id(reloaded.id, &mut conn)
        .await
        .unwrap()
        .projection
        .expect("competition section");
    let TournamentFormatResponse::Elimination(EliminationState {
        nodes: projected_nodes,
        player_results: projected_player_results,
        ..
    }) = snapshot.format
    else {
        panic!("elimination projection")
    };
    assert_eq!(
        projected_player_results
            .iter()
            .map(|player| player.player)
            .collect::<Vec<_>>(),
        setup.seeded_players
    );
    assert!(projected_nodes
        .iter()
        .any(|node| node.entrants.contains(&Some(placement[0]))));
    let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    for (seed, id) in setup.seeded_players.iter().enumerate() {
        assert_eq!(
            memberships
                .iter()
                .find(|row| row.user_id == *id)
                .unwrap()
                .pairing_number,
            Some(seed as i32)
        );
    }
    assert!(matches!(
        fixed_field::confirm_elimination_start(
            tournament.id,
            owner.id,
            setup.id,
            setup.seeded_players,
            &mut conn
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_setup_unlocks_admission_and_stale_commands_cannot_control_its_replacement() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
    let prepared =
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players.clone(), &mut conn)
            .await
            .unwrap();
    let mut expired = prepared.start_setup().unwrap();
    expired.opened_at = Utc::now() - Duration::minutes(15);
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::start_setup.eq(Some(serde_json::to_value(&expired).unwrap())))
        .execute(&mut conn)
        .await
        .unwrap();
    assert!(matches!(
        fixed_field::confirm_elimination_start(
            tournament.id,
            owner.id,
            expired.id,
            expired.seeded_players.clone(),
            &mut conn
        )
        .await,
        Err(DbError::InvalidInput { .. })
    ));
    let unlocked = Tournament::find(tournament.id, &mut conn).await.unwrap();
    unlocked.leave(&players[0], &mut conn).await.unwrap();
    let cleared = fixed_field::expire_elimination_setups(Utc::now(), 100, &mut conn)
        .await
        .unwrap();
    assert_eq!(cleared.len(), 1);
    assert!(cleared[0].start_setup().is_none());
    let replacement = fixed_field::prepare_elimination_start(
        tournament.id,
        owner.id,
        players[1..].to_vec(),
        &mut conn,
    )
    .await
    .unwrap()
    .start_setup()
    .unwrap();
    assert!(matches!(
        fixed_field::cancel_elimination_start(tournament.id, owner.id, expired.id, &mut conn).await,
        Err(DbError::InvalidInput { .. })
    ));
    assert!(matches!(
        fixed_field::confirm_elimination_start(
            tournament.id,
            owner.id,
            expired.id,
            expired.seeded_players,
            &mut conn
        )
        .await,
        Err(DbError::InvalidInput { .. })
    ));
    assert!(
        fixed_field::expire_elimination_setups(Utc::now(), 100, &mut conn)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        Tournament::find(tournament.id, &mut conn)
            .await
            .unwrap()
            .start_setup()
            .unwrap()
            .id,
        replacement.id
    );
    fixed_field::cancel_elimination_start(tournament.id, owner.id, replacement.id, &mut conn)
        .await
        .unwrap();
    assert!(Tournament::find(tournament.id, &mut conn)
        .await
        .unwrap()
        .start_setup()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn deleted_frozen_entrant_keeps_position_and_is_withdrawn_at_start_without_games() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
    let prepared =
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players.clone(), &mut conn)
            .await
            .unwrap();
    let setup = prepared.start_setup().unwrap();
    let deleted_id = setup.seeded_players[2];
    let player = User::find_by_uuid(&deleted_id, &mut conn).await.unwrap();
    player
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .unwrap();
    let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(memberships.len(), 3);
    assert!(memberships
        .iter()
        .find(|row| row.user_id == deleted_id)
        .unwrap()
        .withdrawn_at
        .is_none());
    fixed_field::cancel_elimination_start(tournament.id, owner.id, setup.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .len(),
        3
    );
    let prepared =
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players, &mut conn)
            .await
            .unwrap();
    let setup = prepared.start_setup().unwrap();
    let started = fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        setup.id,
        setup.seeded_players,
        &mut conn,
    )
    .await
    .unwrap();
    assert!(started
        .games
        .iter()
        .all(|game| !game.user_is_player(deleted_id)));
    assert_eq!(
        games::table
            .filter(games::tournament_id.eq(Some(tournament.id)))
            .filter(
                games::white_id
                    .eq(deleted_id)
                    .or(games::black_id.eq(deleted_id))
            )
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
    let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(memberships.len(), 3);
    let deleted = memberships
        .iter()
        .find(|row| row.user_id == deleted_id)
        .unwrap();
    assert_eq!(deleted.pairing_number, Some(2));
    assert!(deleted
        .withdrawn_at
        .is_some_and(|at| at >= deleted.accepted_at));
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_start_attempts_acquire_only_one_setup_owner() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
    let admin = create_user("setup_other_admin", &mut conn).await;
    diesel::update(users::table.find(admin.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();
    let mut other_conn = get_conn(&db.pool).await.unwrap();
    let (first, second) = tokio::join!(
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players.clone(), &mut conn),
        fixed_field::prepare_elimination_start(tournament.id, admin.id, players, &mut other_conn),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let winner = first.or(second).unwrap();
    assert_eq!(
        Tournament::find(tournament.id, &mut conn)
            .await
            .unwrap()
            .start_setup(),
        winner.start_setup()
    );
    assert_eq!(
        games::table
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_an_admin_without_membership_cannot_leave_their_concurrent_setup_locked() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, _, players) = fixture(&mut conn).await;
    let admin = create_user("setup_delete_admin", &mut conn).await;
    diesel::update(users::table.find(admin.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();
    let pool = db.pool.clone();
    let tournament_id = tournament.id;
    let admin_id = admin.id;
    let deletion = conn
        .transaction::<_, DbError, _>(async move |tc| {
            users::table
                .find(admin_id)
                .select(users::id)
                .for_no_key_update()
                .first::<Uuid>(tc)
                .await?;
            let mut deleting_conn = pool.get_owned().await.unwrap();
            let backend = diesel::sql_query("SELECT pg_backend_pid() AS pid")
                .get_result::<common::locks::Backend>(&mut deleting_conn)
                .await?;
            let deletion = tokio::spawn(async move {
                admin
                    .soft_delete("replacement-password-hash", &mut deleting_conn)
                    .await
            });
            common::locks::wait_for_backend_lock(&pool, backend.pid).await;
            let mut preparing_conn = get_conn(&pool).await.unwrap();
            let prepared = fixed_field::prepare_elimination_start(
                tournament_id,
                admin_id,
                players,
                &mut preparing_conn,
            )
            .await?;
            assert_eq!(prepared.start_setup().unwrap().owner_id, admin_id);
            Ok(deletion)
        })
        .await
        .expect("setup commits after deletion's initial setup-owner scan");
    tokio::time::timeout(std::time::Duration::from_secs(10), deletion)
        .await
        .unwrap()
        .unwrap()
        .expect("deletion retries and clears the concurrent setup");
    assert!(
        User::find_by_uuid(&admin_id, &mut conn)
            .await
            .unwrap()
            .deleted
    );
    let tournament = Tournament::find(tournament.id, &mut conn).await.unwrap();
    assert!(tournament.start_setup().is_none());
    assert_eq!(tournament.status(), TournamentStatus::NotStarted);
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_setup_owner_removes_their_roster_membership_when_a_coorganizer_remains() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, mut players) = fixture(&mut conn).await;
    let coorganizer = create_user("setup_del_coorg", &mut conn).await;
    tournament
        .invite_organizer(owner.id, coorganizer.id, &mut conn)
        .await
        .unwrap();
    tournament
        .accept_organizer_invitation(coorganizer.id, &mut conn)
        .await
        .unwrap();
    tournament.join(&owner.id, &mut conn).await.unwrap();
    players.push(owner.id);
    fixed_field::prepare_elimination_start(tournament.id, owner.id, players, &mut conn)
        .await
        .unwrap();
    owner
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .unwrap();
    let reloaded = Tournament::find(tournament.id, &mut conn).await.unwrap();
    assert!(reloaded.start_setup().is_none());
    assert_eq!(reloaded.status(), TournamentStatus::NotStarted);
    let members = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(members.len(), 3);
    assert!(members.iter().all(|member| member.user_id != owner.id));
}

#[tokio::test(flavor = "multi_thread")]
async fn all_deleted_frozen_entrants_are_withdrawn_together_before_start_releases_games() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
    let prepared =
        fixed_field::prepare_elimination_start(tournament.id, owner.id, players.clone(), &mut conn)
            .await
            .unwrap();
    for player in players {
        User::find_by_uuid(&player, &mut conn)
            .await
            .unwrap()
            .soft_delete("replacement-password-hash", &mut conn)
            .await
            .unwrap();
    }
    let setup = prepared.start_setup().unwrap();
    let started = fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        setup.id,
        setup.seeded_players,
        &mut conn,
    )
    .await
    .unwrap();
    assert!(started.games.is_empty());
    assert_eq!(
        games::table
            .filter(games::tournament_id.eq(Some(tournament.id)))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
    let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(memberships.len(), 3);
    assert!(memberships.iter().all(|membership| membership
        .withdrawn_at
        .is_some_and(|at| at >= membership.accepted_at)));
    assert_eq!(started.tournament.status(), TournamentStatus::Finished);
}
