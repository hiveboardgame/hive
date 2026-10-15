mod common;

use chrono::{Duration, Utc};
use common::tournament::{
    create_user,
    insert_unfrozen_memberships,
    realtime_clock,
    round_robin_configuration,
};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{NewTournament, Tournament, TournamentSlot, TournamentUser},
    schema::{tournaments, users},
    tournaments::fixed_field,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::{
    tournament::{swiss::Config as SwissConfig, BotAdmission, Config, FormatConfig},
    TournamentDetails,
    TournamentStatus,
};
use uuid::Uuid;

async fn fixture(
    prefix: &str,
    swiss: bool,
    scheduled: bool,
    conn: &mut DbConn<'_>,
) -> (Tournament, Uuid, Vec<Uuid>) {
    let organizer = create_user(&format!("{prefix}_o"), conn).await;
    let minimum = if swiss { 5 } else { 4 };
    let configuration = if swiss {
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(SwissConfig::automatic_swiss(0, realtime_clock())),
        }
    } else {
        round_robin_configuration()
    };
    let tournament = Tournament::create(
        organizer.id,
        &NewTournament::new(TournamentDetails {
            name: format!("{prefix} start cleanup"),
            description: None,
            seats: Some(8),
            min_seats: minimum,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: scheduled.then(|| Utc::now() + Duration::hours(1)),
            configuration,
        })
        .expect("valid start fixture"),
        conn,
    )
    .await
    .expect("create start fixture");
    let mut entrants = Vec::new();
    for index in 0..minimum {
        entrants.push(create_user(&format!("{prefix}_p{index}"), conn).await.id);
    }
    insert_unfrozen_memberships(tournament.id, &entrants, Utc::now(), conn).await;
    // Imported or inconsistent data can retain a membership for an already
    // anonymized account; ordinary account deletion normally removes it.
    diesel::update(users::table.find(*entrants.last().unwrap()))
        .set(users::deleted.eq(true))
        .execute(conn)
        .await
        .expect("retain deleted account on imported roster");
    (tournament, organizer.id, entrants)
}

#[tokio::test(flavor = "multi_thread")]
async fn manual_start_commits_deleted_roster_cleanup_and_can_start_after_another_join() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    for (prefix, swiss) in [("manual_rr", false), ("manual_swiss", true)] {
        let (tournament, organizer, mut entrants) = fixture(prefix, swiss, false, &mut conn).await;
        let deleted_id = *entrants.last().unwrap();
        let outcome =
            fixed_field::start_by_organizer(tournament.id, organizer, entrants.clone(), &mut conn)
                .await
                .expect("cleanup below-minimum roster without rolling it back");
        assert!(!outcome.started_now);
        assert!(outcome.games.is_empty());
        let persisted = Tournament::find(tournament.id, &mut conn).await.unwrap();
        assert_eq!(persisted.status(), TournamentStatus::NotStarted);
        assert!(persisted.starts_at.is_none());
        assert!(
            TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .unwrap()
                .is_empty()
        );
        let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap();
        assert_eq!(memberships.len(), entrants.len() - 1);
        assert!(memberships.iter().all(|row| row.user_id != deleted_id));
        assert!(memberships.iter().all(|row| row.pairing_number.is_none()));

        let replacement = create_user(&format!("{prefix}_new"), &mut conn).await;
        persisted.join(&replacement.id, &mut conn).await.unwrap();
        entrants.pop();
        entrants.push(replacement.id);
        let outcome =
            fixed_field::start_by_organizer(tournament.id, organizer, entrants, &mut conn)
                .await
                .expect("start after active roster reaches minimum");
        assert!(outcome.started_now);
        assert!(!outcome.games.is_empty());
        assert!(outcome
            .games
            .iter()
            .all(|game| !game.user_is_player(deleted_id)));
        assert!(
            TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .unwrap()
                .iter()
                .all(|slot| slot.white != deleted_id && slot.black != deleted_id)
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn scheduled_start_removes_deleted_entrants_before_applying_the_minimum() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    for (prefix, swiss) in [("scheduled_rr", false), ("scheduled_swiss", true)] {
        let (tournament, _, entrants) = fixture(prefix, swiss, true, &mut conn).await;
        diesel::update(tournaments::table.find(tournament.id))
            .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::minutes(1))))
            .execute(&mut conn)
            .await
            .unwrap();
        let outcome = fixed_field::start_scheduled(tournament.id, &mut conn)
            .await
            .expect("switch undersubscribed active roster to manual start");
        assert!(!outcome.started_now);
        assert!(outcome.games.is_empty());
        let persisted = Tournament::find(tournament.id, &mut conn).await.unwrap();
        assert_eq!(persisted.status(), TournamentStatus::NotStarted);
        assert!(persisted.starts_at.is_none());
        let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap();
        assert_eq!(memberships.len(), entrants.len() - 1);
        assert!(memberships
            .iter()
            .all(|row| row.user_id != *entrants.last().unwrap()));
        assert!(persisted.games(&mut conn).await.unwrap().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sufficient_active_rosters_start_without_creating_games_for_deleted_entrants() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    for (prefix, swiss) in [("ready_rr", false), ("ready_swiss", true)] {
        let (tournament, organizer, mut entrants) = fixture(prefix, swiss, false, &mut conn).await;
        let deleted_id = *entrants.last().unwrap();
        let extra = create_user(&format!("{prefix}_extra"), &mut conn).await;
        insert_unfrozen_memberships(tournament.id, &[extra.id], Utc::now(), &mut conn).await;
        entrants.push(extra.id);
        let outcome =
            fixed_field::start_by_organizer(tournament.id, organizer, entrants.clone(), &mut conn)
                .await
                .expect("start after excluding deleted entrant");
        assert!(outcome.started_now);
        assert!(!outcome.games.is_empty());
        assert!(outcome
            .games
            .iter()
            .all(|game| !game.user_is_player(deleted_id)));
        let memberships = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap();
        assert_eq!(memberships.len(), entrants.len() - 1);
        assert!(memberships.iter().all(|row| row.user_id != deleted_id));
        assert!(
            TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .unwrap()
                .iter()
                .all(|slot| slot.white != deleted_id && slot.black != deleted_id)
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn start_cleanup_preserves_authorization_and_active_roster_validation() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (tournament, organizer, entrants) = fixture("guard_rr", false, false, &mut conn).await;
    let outsider = create_user("start_outsider", &mut conn).await;
    assert!(matches!(
        fixed_field::start_by_organizer(tournament.id, outsider.id, entrants.clone(), &mut conn)
            .await,
        Err(DbError::Unauthorized)
    ));
    let mut missing_active = entrants.clone();
    missing_active.remove(0);
    assert!(matches!(
        fixed_field::start_by_organizer(tournament.id, organizer, missing_active, &mut conn).await,
        Err(DbError::InvalidAction { .. })
    ));
    let mut duplicate_deleted = entrants.clone();
    duplicate_deleted.push(*entrants.last().unwrap());
    assert!(matches!(
        fixed_field::start_by_organizer(tournament.id, organizer, duplicate_deleted, &mut conn)
            .await,
        Err(DbError::InvalidAction { .. })
    ));
    assert_eq!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .len(),
        entrants.len()
    );
}
