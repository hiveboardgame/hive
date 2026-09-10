use shared_types::tournament_view::TournamentFormatResponse;
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
use diesel_async::RunQueryDsl;
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
async fn setup_is_exclusive_even_against_an_admin_and_freezes_rating_seeds() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, owner, players) = fixture(&mut conn).await;
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
    assert!(fixed_field::prepare_elimination_start(
        tournament.id,
        admin.id,
        players.clone(),
        &mut conn
    )
    .await
    .is_err());
    assert!(
        fixed_field::cancel_elimination_start(tournament.id, admin.id, setup.id, &mut conn)
            .await
            .is_err()
    );
    assert!(fixed_field::confirm_elimination_start(
        tournament.id,
        admin.id,
        setup.id,
        setup.seeded_players.clone(),
        &mut conn
    )
    .await
    .is_err());
    assert!(prepared.join(&admin.id, &mut conn).await.is_err());
    assert!(prepared.leave(&players[0], &mut conn).await.is_err());
    let mut invalid = setup.seeded_players.clone();
    invalid[1] = invalid[0];
    assert!(fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        setup.id,
        invalid,
        &mut conn
    )
    .await
    .is_err());
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
    let snapshot = load_by_id(reloaded.id, &mut conn).await.unwrap();
    let TournamentFormatResponse::Elimination {
        nodes: projected_nodes,
        player_results: projected_player_results,
        ..
    } = snapshot.format
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
    assert!(fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        setup.id,
        setup.seeded_players,
        &mut conn
    )
    .await
    .is_err());
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
    assert!(fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        expired.id,
        expired.seeded_players.clone(),
        &mut conn
    )
    .await
    .is_err());
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
    assert!(
        fixed_field::cancel_elimination_start(tournament.id, owner.id, expired.id, &mut conn)
            .await
            .is_err()
    );
    assert!(fixed_field::confirm_elimination_start(
        tournament.id,
        owner.id,
        expired.id,
        expired.seeded_players,
        &mut conn
    )
    .await
    .is_err());
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
async fn deleted_locked_entrant_survives_cancellation_and_can_start_then_be_withdrawn() {
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
        .any(|game| game.user_is_player(deleted_id)));
    assert!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .iter()
            .find(|row| row.user_id == deleted_id)
            .unwrap()
            .withdrawn_at
            .is_none()
    );
    let withdrawn = fixed_field::withdraw_player(tournament.id, deleted_id, owner.id, &mut conn)
        .await
        .unwrap();
    assert!(withdrawn.applied);
    assert!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .iter()
            .find(|row| row.user_id == deleted_id)
            .unwrap()
            .withdrawn_at
            .is_some()
    );
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
    let mut other_conn = get_conn(&db.pool).await.unwrap();
    let (preparation, deletion) = tokio::join!(
        fixed_field::prepare_elimination_start(tournament.id, admin.id, players, &mut conn),
        admin.soft_delete("replacement-password-hash", &mut other_conn),
    );
    let current_admin = User::find_by_uuid(&admin.id, &mut conn).await.unwrap();
    let current_tournament = Tournament::find(tournament.id, &mut conn).await.unwrap();
    assert!(!current_admin.deleted || current_tournament.start_setup().is_none());
    assert!(
        preparation.is_ok()
            || current_admin.deleted
            || matches!(preparation, Err(DbError::SerializationConflict))
    );
    match deletion {
        Ok(_) => assert!(current_admin.deleted),
        Err(DbError::SerializationConflict) => {
            // The bounded retry policy may reject the overlapping deletion.
            // It must roll back completely and allow a fresh command afterward.
            assert!(!current_admin.deleted);
            admin
                .soft_delete("replacement-password-hash", &mut other_conn)
                .await
                .unwrap();
        }
        Err(error) => panic!("unexpected account deletion error: {error}"),
    }
    let tournament = Tournament::find(tournament.id, &mut conn).await.unwrap();
    assert!(tournament.start_setup().is_none());
    assert_eq!(tournament.status(), TournamentStatus::NotStarted);
}
