#![cfg(feature = "ssr")]

mod common;

use apis::responses::{
    ProfileTournamentResult,
    ProfileTournamentsPage,
    TournamentAccessState,
    TOURNAMENT_BROWSE_PAGE_SIZE,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use db_lib::{
    get_conn,
    models::{
        NewTournament,
        NewUser,
        Tournament,
        TournamentInvitation,
        TournamentOrganizer,
        TournamentUser,
        User,
    },
    schema::{tournament_final_outcomes, tournaments, tournaments_organizers, tournaments_users},
    DbConn,
};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use shared_types::{
    tournament::{
        round_robin::Config as RoundRobinConfig,
        standings::{Group, Placement, Row, Snapshot, Value},
        BotAdmission,
        Config,
        FormatConfig,
    },
    Clock,
    RealtimeClock,
    TournamentDetails,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn instant() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2099, 5, 10, 12, 0, 0)
        .single()
        .unwrap()
}

fn configuration() -> Config {
    Config {
        bot_admission: BotAdmission::HumansOnly,
        format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
            NonZeroU32::new(1).unwrap(),
            Clock::Realtime(RealtimeClock {
                base_seconds: NonZeroU32::new(60).unwrap(),
                increment_seconds: 0,
            }),
        )),
    }
}

async fn insert_tournament(
    name: &str,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    conn: &mut DbConn<'_>,
) -> Tournament {
    let mut new = NewTournament::new(TournamentDetails {
        name: name.to_owned(),
        description: None,
        seats: Some(16),
        min_seats: 2,
        invite_only: false,
        band_lower: None,
        band_upper: None,
        starts_at: None,
        configuration: configuration(),
    })
    .expect("validate profile tournament fixture");
    new.created_at = instant();
    new.started_at = match finished_at {
        Some(finished_at) => Some(finished_at - Duration::hours(1)),
        None => started_at,
    };
    new.finished_at = finished_at;
    diesel::insert_into(tournaments::table)
        .values(new)
        .get_result(conn)
        .await
        .expect("insert profile tournament fixture")
}

async fn create_user(name: &str, conn: &mut DbConn<'_>) -> User {
    User::create(
        NewUser::new(name, "password", &format!("{name}@example.test")).unwrap(),
        conn,
    )
    .await
    .unwrap()
}

async fn enter(tournament: &Tournament, user_id: Uuid, withdrawn: bool, conn: &mut DbConn<'_>) {
    let mut entrant = TournamentUser::accepted_at(tournament.id, user_id, instant());
    entrant.withdrawn_at = withdrawn.then(instant);
    diesel::insert_into(tournaments_users::table)
        .values(entrant)
        .execute(conn)
        .await
        .unwrap();
}

async fn organize(tournament: &Tournament, user_id: Uuid, conn: &mut DbConn<'_>) {
    diesel::insert_into(tournaments_organizers::table)
        .values(TournamentOrganizer::new(tournament.id, user_id))
        .execute(conn)
        .await
        .unwrap();
}

fn standings_row(user_id: Uuid) -> Row {
    Row {
        user_id,
        primary_score: Value::Integer(0),
        games_played: 0,
        matches_played: None,
        wins: 0,
        draws: 0,
        losses: 0,
        counts: Vec::new(),
        values: Vec::new(),
    }
}

async fn record_outcome(tournament: &Tournament, groups: &[Vec<Uuid>], conn: &mut DbConn<'_>) {
    let snapshot = Snapshot {
        groups: groups
            .iter()
            .enumerate()
            .map(|(index, members)| Group {
                placement: Placement::CompetitionRank(index as u32 + 1),
                rows: members.iter().copied().map(standings_row).collect(),
                separated_by: None,
            })
            .collect(),
    };
    diesel::insert_into(tournament_final_outcomes::table)
        .values((
            tournament_final_outcomes::tournament_id.eq(tournament.id),
            tournament_final_outcomes::standings.eq(serde_json::to_value(&snapshot).unwrap()),
            tournament_final_outcomes::finish_reason.eq("completed"),
        ))
        .execute(conn)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn profile_page_groups_roles_results_and_owner_only_invitations() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let alice = create_user("profile_alice", &mut conn).await.id;
    let bob = create_user("profile_bob", &mut conn).await.id;
    let carol = create_user("profile_carol", &mut conn).await.id;

    let upcoming = insert_tournament("Upcoming entrant", None, None, &mut conn).await;
    enter(&upcoming, alice, false, &mut conn).await;
    let running_organized =
        insert_tournament("Running organized", Some(instant()), None, &mut conn).await;
    organize(&running_organized, alice, &mut conn).await;
    let running_both = insert_tournament(
        "Running both",
        Some(instant() + Duration::hours(1)),
        None,
        &mut conn,
    )
    .await;
    enter(&running_both, alice, false, &mut conn).await;
    organize(&running_both, alice, &mut conn).await;

    let ranked = insert_tournament(
        "Finished ranked",
        Some(instant()),
        Some(instant()),
        &mut conn,
    )
    .await;
    for user in [alice, bob, carol] {
        enter(&ranked, user, false, &mut conn).await;
    }
    record_outcome(&ranked, &[vec![bob], vec![alice], vec![carol]], &mut conn).await;
    let withdrawn = insert_tournament(
        "Finished withdrawn",
        Some(instant()),
        Some(instant() - Duration::days(1)),
        &mut conn,
    )
    .await;
    enter(&withdrawn, alice, true, &mut conn).await;
    enter(&withdrawn, bob, false, &mut conn).await;
    record_outcome(&withdrawn, &[vec![bob], vec![alice]], &mut conn).await;
    let organized = insert_tournament(
        "Finished organized",
        Some(instant()),
        Some(instant() - Duration::days(2)),
        &mut conn,
    )
    .await;
    organize(&organized, alice, &mut conn).await;
    enter(&organized, bob, false, &mut conn).await;
    record_outcome(&organized, &[vec![bob]], &mut conn).await;
    let unrecorded = insert_tournament(
        "Finished without outcome",
        Some(instant()),
        Some(instant() - Duration::days(3)),
        &mut conn,
    )
    .await;
    enter(&unrecorded, alice, false, &mut conn).await;

    let unrelated = insert_tournament("Unrelated", None, None, &mut conn).await;
    enter(&unrelated, bob, false, &mut conn).await;
    let invitation = insert_tournament("Invitation pending", None, None, &mut conn).await;
    TournamentInvitation::new(invitation.id, alice)
        .insert(&mut conn)
        .await
        .unwrap();

    let page = ProfileTournamentsPage::load(alice, Some(bob), 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        page.active
            .iter()
            .map(|entry| entry.card.name.as_str())
            .collect::<Vec<_>>(),
        ["Running both", "Running organized", "Upcoming entrant"]
    );
    assert!(page.active[0].role.playing && page.active[0].role.organizing);
    assert!(!page.active[1].role.playing && page.active[1].role.organizing);
    assert!(page.active[2].role.playing && !page.active[2].role.organizing);
    assert!(page.active.iter().all(|entry| entry.result.is_none()));
    assert_eq!(
        page.active[2].card.summary.access,
        Some(TournamentAccessState::Open)
    );
    assert_eq!(
        page.finished
            .iter()
            .map(|entry| entry.card.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Finished ranked",
            "Finished withdrawn",
            "Finished organized",
            "Finished without outcome",
        ]
    );
    assert_eq!(
        page.finished[0].result,
        Some(ProfileTournamentResult::Ranked { rank: 2, total: 3 })
    );
    assert_eq!(
        page.finished[1].result,
        Some(ProfileTournamentResult::Withdrawn)
    );
    assert_eq!(page.finished[2].result, None);
    assert!(page.finished[2].role.organizing && !page.finished[2].role.playing);
    assert_eq!(page.finished[3].result, None);
    assert_eq!(page.finished_total, 4);
    assert_eq!(page.page, 1);
    assert_eq!(page.invitations, None);

    let own_page = ProfileTournamentsPage::load(alice, Some(alice), 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        own_page.invitations,
        Some(vec![String::from("Invitation pending")])
    );
    assert_eq!(
        own_page.active[2].card.summary.access,
        Some(TournamentAccessState::Joined)
    );

    let guest_page = ProfileTournamentsPage::load(alice, None, 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(guest_page.invitations, None);
    assert_eq!(guest_page.active.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn finished_tournaments_page_and_clamp_while_active_stay_complete() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let alice = create_user("profile_paging", &mut conn).await.id;
    let active = insert_tournament("Still running", Some(instant()), None, &mut conn).await;
    enter(&active, alice, false, &mut conn).await;
    for index in 0..=TOURNAMENT_BROWSE_PAGE_SIZE {
        let tournament = insert_tournament(
            &format!("Finished {index}"),
            Some(instant()),
            Some(instant() - Duration::minutes(index as i64)),
            &mut conn,
        )
        .await;
        enter(&tournament, alice, false, &mut conn).await;
    }

    let first = ProfileTournamentsPage::load(alice, None, 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(first.finished.len(), TOURNAMENT_BROWSE_PAGE_SIZE);
    assert_eq!(first.finished[0].card.name, "Finished 0");
    assert_eq!(
        first.finished_total as usize,
        TOURNAMENT_BROWSE_PAGE_SIZE + 1
    );
    assert_eq!(first.active.len(), 1);

    let clamped = ProfileTournamentsPage::load(alice, None, 99, &mut conn)
        .await
        .unwrap();
    assert_eq!(clamped.page, 2);
    assert_eq!(clamped.finished.len(), 1);
    assert_eq!(
        clamped.finished[0].card.name,
        format!("Finished {TOURNAMENT_BROWSE_PAGE_SIZE}")
    );
    assert_eq!(clamped.active.len(), 1);
}
