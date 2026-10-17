mod common;

use chrono::{DateTime, Duration, Utc};
use common::tournament::{
    create_rr_tournament,
    create_rr_tournament_with_configuration,
    create_user,
    fixed_instant,
    round_robin_configuration,
    round_robin_configuration_with,
};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{NewTournament, ScheduleOffer, Tournament, TournamentSlot, TournamentUser},
    schema::{
        games,
        schedule_offers,
        tournament_final_outcomes,
        tournament_slots,
        tournaments,
        tournaments_organizers,
        tournaments_users,
    },
    tournaments::{arena, fixed_field},
    DbConn,
};
use diesel::{
    delete,
    prelude::*,
    result::{DatabaseErrorKind, Error as DieselError},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{Color, GameStatus};
use serde_json::{json, Value as JsonValue};
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        swiss::{
            Config as SwissConfig,
            PrimaryScore as DoubleSwissPrimaryScore,
            System as SwissSystem,
        },
        BotAdmission,
        Clock,
        Config,
        FormatConfig,
        RealtimeClock,
        ReleasePolicy,
    },
    Conclusion,
    ScheduleOfferStatus,
    TournamentDetails,
    TournamentGameResult,
    TournamentStatus,
};
use std::{collections::HashSet, num::NonZeroU32};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn typed_creation_is_atomic_and_has_no_premature_tournament_state() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("typed_organizer", &mut conn).await;
    let new_tournament = NewTournament::new(tournament_details("Typed persistence cup"))
        .expect("validate typed tournament details");

    let organizer_id = organizer.id;
    let tournament = conn
        .transaction::<_, DbError, _>(async move |tc| {
            Tournament::create(organizer_id, &new_tournament, tc).await
        })
        .await
        .expect("atomically create tournament and organizer");

    assert_eq!(tournament.status(), TournamentStatus::NotStarted);
    let persisted_configuration = tournaments::table
        .find(tournament.id)
        .select(tournaments::configuration)
        .first::<JsonValue>(&mut conn)
        .await
        .expect("load persisted configuration JSON");
    assert!(persisted_configuration.get("progression").is_none());
    let mut legacy_configuration = persisted_configuration.clone();
    legacy_configuration
        .as_object_mut()
        .expect("configuration is an object")
        .insert(String::from("progression"), json!("automatic"));
    let legacy_update = diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::configuration.eq(legacy_configuration))
        .execute(&mut conn)
        .await;
    assert!(matches!(
        legacy_update,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            _
        ))
    ));
    assert_eq!(
        tournaments_organizers::table
            .filter(tournaments_organizers::tournament_id.eq(tournament.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count organizers"),
        1
    );
    for count in [
        tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(tournament.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count participants"),
        tournament_slots::table
            .filter(tournament_slots::tournament_id.eq(tournament.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count slots"),
        tournament_final_outcomes::table
            .filter(tournament_final_outcomes::tournament_id.eq(tournament.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count final outcomes"),
    ] {
        assert_eq!(count, 0);
    }

    let before_failure = tournaments::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .expect("count tournaments before failed create");
    let rejected_nanoid = {
        let new_tournament = NewTournament::new(tournament_details("Rolled back tournament"))
            .expect("validate rollback fixture");
        let nanoid = new_tournament.nanoid.clone();
        let result = conn
            .transaction::<_, DbError, _>(async move |tc| {
                Tournament::create(Uuid::from_u128(0xdead_beef), &new_tournament, tc).await
            })
            .await;
        assert!(result.is_err());
        nanoid
    };
    assert_eq!(
        tournaments::table
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count tournaments after failed create"),
        before_failure
    );
    assert!(matches!(
        tournaments::table
            .filter(tournaments::nanoid.eq(rejected_nanoid))
            .select(tournaments::id)
            .first::<Uuid>(&mut conn)
            .await,
        Err(DieselError::NotFound)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn selected_double_swiss_primary_score_survives_persistence() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("ds_score_org", &mut conn).await;
    let configuration = Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
            0,
            Clock::Realtime(RealtimeClock {
                base_seconds: NonZeroU32::new(180).unwrap(),
                increment_seconds: 2,
            }),
            DoubleSwissPrimaryScore::MatchPoints,
        )),
    };
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "Double Swiss scoring persistence",
        TournamentStatus::NotStarted,
        None,
        configuration,
        &mut conn,
    )
    .await;

    let persisted_tournament = Tournament::find(tournament.id, &mut conn)
        .await
        .expect("reload persisted Double-Swiss tournament");
    let FormatConfig::Swiss(swiss) = &persisted_tournament.configuration().format else {
        panic!("persisted tournament changed format");
    };
    let SwissSystem::DoubleSwiss(double_swiss) = &swiss.system else {
        panic!("persisted Swiss tournament changed system");
    };
    assert_eq!(
        double_swiss.primary_score,
        DoubleSwissPrimaryScore::MatchPoints,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn tournament_memberships_are_loaded_in_pairing_number_order() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("roster_organizer", &mut conn).await;
    let first = create_user("roster_first", &mut conn).await;
    let second = create_user("roster_second", &mut conn).await;
    let tournament = create_rr_tournament(
        organizer.id,
        "Ordered roster tournament",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let mut later = TournamentUser::accepted_at(tournament.id, first.id, fixed_instant());
    later.pairing_number = Some(2);
    let mut earlier = TournamentUser::accepted_at(tournament.id, second.id, fixed_instant());
    earlier.pairing_number = Some(1);
    insert_membership(&later, &mut conn).await;
    insert_membership(&earlier, &mut conn).await;

    let loaded = TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load ordered memberships");
    assert_eq!(
        loaded
            .iter()
            .map(|membership| membership.user_id)
            .collect::<Vec<_>>(),
        vec![second.id, first.id]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn schedule_offer_candidate_shape_and_single_pending_are_database_invariants() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("schedule_shape", &mut conn).await;
    let first = future_instant(60);
    let second = future_instant(120);
    let third = future_instant(180);
    let fourth = future_instant(240);

    let single_id = insert_pending_offer(
        fixture.tournament_id,
        fixture.slot_id,
        fixture.white_id,
        vec![Some(first)],
        &mut conn,
    )
    .await
    .expect("persist one schedule candidate");
    diesel::delete(schedule_offers::table.find(single_id))
        .execute(&mut conn)
        .await
        .expect("remove one-candidate fixture");

    let three_id = insert_pending_offer(
        fixture.tournament_id,
        fixture.slot_id,
        fixture.white_id,
        vec![Some(first), Some(second), Some(third)],
        &mut conn,
    )
    .await
    .expect("persist three schedule candidates");
    let persisted = ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
        .await
        .expect("load valid candidate fixture");
    assert_eq!(persisted.len(), 1);
    assert_eq!(
        persisted[0].candidates().unwrap(),
        vec![first, second, third]
    );

    let competing_pending = insert_pending_offer(
        fixture.tournament_id,
        fixture.slot_id,
        fixture.black_id,
        vec![Some(fourth)],
        &mut conn,
    )
    .await;
    assert!(matches!(
        competing_pending,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            _
        ))
    ));
    diesel::delete(schedule_offers::table.find(three_id))
        .execute(&mut conn)
        .await
        .expect("remove three-candidate fixture");

    let third_member = create_user("schedule_shape_3", &mut conn).await;
    let membership =
        TournamentUser::accepted_at(fixture.tournament_id, third_member.id, fixed_instant());
    insert_membership(&membership, &mut conn).await;
    let non_entrant_offer = insert_pending_offer(
        fixture.tournament_id,
        fixture.slot_id,
        third_member.id,
        vec![Some(fourth)],
        &mut conn,
    )
    .await;
    assert!(matches!(
        non_entrant_offer,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            _
        ))
    ));

    let invalid_candidates: Vec<Vec<Option<DateTime<Utc>>>> = vec![
        Vec::new(),
        vec![Some(first), Some(second), Some(third), Some(fourth)],
        vec![Some(first), Some(first)],
        vec![Some(first), None],
    ];
    for candidates in invalid_candidates {
        let rejected = insert_pending_offer(
            fixture.tournament_id,
            fixture.slot_id,
            fixture.white_id,
            candidates,
            &mut conn,
        )
        .await;
        assert!(matches!(
            rejected,
            Err(DieselError::DatabaseError(
                DatabaseErrorKind::CheckViolation,
                _
            ))
        ));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_opponent_can_accept_an_exact_candidate_and_rescheduling_keeps_the_appointment() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("schedule_accept", &mut conn).await;
    let outsider = create_user("schedule_accept_x", &mut conn).await;
    let first = future_instant(60);
    let selected = future_instant(120);
    let rescheduled = future_instant(180);
    let offer = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![first, selected],
        &mut conn,
    )
    .await
    .expect("propose candidate set")
    .pop()
    .expect("new pending offer is returned");

    assert!(matches!(
        ScheduleOffer::accept(offer.id, fixture.white_id, selected, &mut conn).await,
        Err(DbError::Unauthorized)
    ));
    assert!(matches!(
        ScheduleOffer::accept(offer.id, outsider.id, selected, &mut conn).await,
        Err(DbError::Unauthorized)
    ));
    assert!(matches!(
        ScheduleOffer::accept(offer.id, fixture.black_id, rescheduled, &mut conn,).await,
        Err(DbError::InvalidAction { .. })
    ));

    let accepted = ScheduleOffer::accept(offer.id, fixture.black_id, selected, &mut conn)
        .await
        .expect("opponent accepts an offered candidate");
    assert_eq!(accepted.status().unwrap(), ScheduleOfferStatus::Accepted);
    assert_eq!(accepted.selected_time, Some(selected));
    assert_eq!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .expect("load accepted appointment")
            .scheduled_at,
        Some(selected)
    );

    let reschedule = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![rescheduled],
        &mut conn,
    )
    .await
    .expect("propose a reschedule")
    .pop()
    .expect("pending reschedule is returned");
    assert_eq!(reschedule.status().unwrap(), ScheduleOfferStatus::Pending);
    assert_eq!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .expect("load appointment while reschedule is pending")
            .scheduled_at,
        Some(selected)
    );
    let history = ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
        .await
        .expect("load appointment history");
    assert_eq!(history.len(), 2);
    assert_eq!(
        history
            .iter()
            .filter(|offer| offer.status().unwrap() == ScheduleOfferStatus::Pending)
            .count(),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn finished_game_rejects_scheduling_even_before_slot_reconciliation() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("schedule_unsealed", &mut conn).await;
    let game_id = games::table
        .filter(games::tournament_id.eq(fixture.tournament_id))
        .filter(games::tournament_slot_id.eq(fixture.slot_id))
        .select(games::id)
        .get_result::<Uuid>(&mut conn)
        .await
        .expect("load Slot-owned game");
    let finished_at = Utc::now();
    diesel::update(games::table.find(game_id))
        .set((
            games::finished.eq(true),
            games::rated.eq(false),
            games::game_status.eq(GameStatus::Adjudicated.to_string()),
            games::tournament_game_result
                .eq(TournamentGameResult::Winner(Color::White).to_string()),
            games::conclusion.eq(Conclusion::Committee.to_string()),
            games::updated_at.eq(finished_at),
            games::finished_at.eq(Some(finished_at)),
        ))
        .execute(&mut conn)
        .await
        .expect("finish Game without resolving its Slot");

    let result = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![future_instant(60)],
        &mut conn,
    )
    .await;
    assert!(matches!(result, Err(DbError::InvalidAction { .. })));
    assert!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .unwrap()
            .resolution
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_slots_retain_offer_history_and_tournament_finish_purges_it() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture_with_configuration(
        "schedule_terminal",
        round_robin_configuration_with(
            NonZeroU32::new(2).unwrap(),
            ReleasePolicy::SequentialPerMatchup,
        ),
        &mut conn,
    )
    .await;
    let selected = future_instant(60);
    let rescheduled = future_instant(120);
    let deadline = future_instant(180);
    let accepted_offer = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![selected],
        &mut conn,
    )
    .await
    .expect("propose initial appointment")
    .pop()
    .unwrap();
    ScheduleOffer::accept(accepted_offer.id, fixture.black_id, selected, &mut conn)
        .await
        .expect("accept initial appointment");
    ScheduleOffer::propose(
        fixture.black_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![rescheduled],
        &mut conn,
    )
    .await
    .expect("propose pending reschedule");
    TournamentSlot::set_deadline_for_slots(
        fixture.tournament_id,
        &[fixture.slot_id],
        Some(deadline),
        &mut conn,
    )
    .await
    .expect("set finish-purge deadline fixture");

    fixed_field::adjudicate_slot_atomic(
        fixture.tournament_id,
        fixture.slot_id,
        TournamentGameResult::Winner(Color::White),
        None,
        fixture.organizer_id,
        &mut conn,
    )
    .await
    .expect("resolve scheduled Slot");
    let terminal_slot = TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
        .await
        .expect("load terminal Slot");
    assert!(terminal_slot.resolution.is_some());
    assert!(terminal_slot.scheduled_at.is_none());
    let history = ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
        .await
        .expect("load retained terminal history");
    assert_eq!(history.len(), 2);
    assert!(history
        .iter()
        .any(|offer| offer.status().unwrap() == ScheduleOfferStatus::Accepted));
    assert!(history
        .iter()
        .any(|offer| offer.status().unwrap() == ScheduleOfferStatus::Cancelled));
    assert!(!history
        .iter()
        .any(|offer| offer.status().unwrap() == ScheduleOfferStatus::Pending));

    let remaining_slots = TournamentSlot::find_by_tournament_id(fixture.tournament_id, &mut conn)
        .await
        .expect("load remaining Slots");
    for slot in remaining_slots
        .into_iter()
        .filter(|slot| slot.resolution.is_none())
    {
        fixed_field::adjudicate_slot_atomic(
            fixture.tournament_id,
            slot.id,
            TournamentGameResult::Winner(Color::White),
            None,
            fixture.organizer_id,
            &mut conn,
        )
        .await
        .expect("resolve remaining Slot before finish");
    }
    assert_eq!(
        Tournament::find(fixture.tournament_id, &mut conn)
            .await
            .expect("reload automatically finished Round Robin")
            .status(),
        TournamentStatus::Finished
    );
    assert!(
        ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
            .await
            .expect("load purged schedule history")
            .is_empty()
    );
    assert!(
        TournamentSlot::find_by_tournament_id(fixture.tournament_id, &mut conn)
            .await
            .expect("load finish-purged Slot metadata")
            .iter()
            .all(|slot| slot.scheduled_at.is_none() && slot.deadline_at.is_none())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_includes_agreed_games_beyond_a_week_in_time_order() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let later = started_schedule_fixture("calendar_later", &mut conn).await;
    let sooner = started_schedule_fixture("calendar_sooner", &mut conn).await;
    let later_time = future_instant(21 * 24 * 60);
    let sooner_time = future_instant(2 * 24 * 60);
    for (fixture, time) in [(&later, later_time), (&sooner, sooner_time)] {
        let offer = ScheduleOffer::propose(
            fixture.white_id,
            fixture.tournament_id,
            fixture.slot_id,
            vec![time],
            &mut conn,
        )
        .await
        .expect("propose a future game")
        .pop()
        .unwrap();
        ScheduleOffer::accept(offer.id, fixture.black_id, time, &mut conn)
            .await
            .expect("agree the game time");
    }
    let upcoming = ScheduleOffer::get_upcoming_agreed_games(Utc::now(), &mut conn)
        .await
        .expect("load calendar");
    assert_eq!(
        upcoming.iter().map(|(_, time)| *time).collect::<Vec<_>>(),
        vec![sooner_time, later_time]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn homepage_keeps_unstarted_agreed_games_through_thirty_minutes_overdue() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("calendar_grace", &mut conn).await;
    let scheduled_at = future_instant(60);
    let offer = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![scheduled_at],
        &mut conn,
    )
    .await
    .expect("propose game")
    .pop()
    .unwrap();
    ScheduleOffer::accept(offer.id, fixture.black_id, scheduled_at, &mut conn)
        .await
        .expect("agree game time");
    let game_id = games::table
        .filter(games::tournament_slot_id.eq(fixture.slot_id))
        .select(games::id)
        .first::<Uuid>(&mut conn)
        .await
        .expect("load scheduled game");

    for elapsed in [
        Duration::zero(),
        Duration::minutes(29),
        Duration::minutes(30),
    ] {
        assert_eq!(
            ScheduleOffer::get_upcoming_agreed_games(scheduled_at + elapsed, &mut conn)
                .await
                .expect("load games within grace window"),
            vec![(game_id, scheduled_at)]
        );
    }
    assert!(ScheduleOffer::get_upcoming_agreed_games(
        scheduled_at + Duration::minutes(30) + Duration::seconds(1),
        &mut conn,
    )
    .await
    .expect("load games after grace window")
    .is_empty());

    diesel::update(games::table.find(game_id))
        .set(games::turn.eq(1))
        .execute(&mut conn)
        .await
        .expect("advance scheduled game");
    assert!(ScheduleOffer::get_upcoming_agreed_games(
        scheduled_at + Duration::minutes(10),
        &mut conn,
    )
    .await
    .expect("load games after play begins")
    .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn deadline_writes_have_no_tournament_or_schedule_lifecycle_side_effects() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("schedule_deadline", &mut conn).await;
    let candidate = future_instant(60);
    let deadline = future_instant(120);
    let offer = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![candidate],
        &mut conn,
    )
    .await
    .expect("propose deadline-isolation fixture")
    .pop()
    .unwrap();
    let before = Tournament::find(fixture.tournament_id, &mut conn)
        .await
        .expect("load tournament before deadline write");

    let updated = TournamentSlot::set_deadline_for_slots(
        fixture.tournament_id,
        &[fixture.slot_id],
        Some(deadline),
        &mut conn,
    )
    .await
    .expect("set scheduling deadline");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].deadline_at, Some(deadline));
    assert!(updated[0].resolution.is_none());
    assert!(updated[0].scheduled_at.is_none());
    let unchanged_offer = ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
        .await
        .expect("load offer after deadline write")
        .pop()
        .unwrap();
    assert_eq!(unchanged_offer.id, offer.id);
    assert_eq!(
        unchanged_offer.status().unwrap(),
        ScheduleOfferStatus::Pending
    );
    let after = Tournament::find(fixture.tournament_id, &mut conn)
        .await
        .expect("load tournament after deadline write");
    assert_eq!(after.status(), TournamentStatus::InProgress);
    assert_eq!(after.started_at, before.started_at);
    assert_eq!(after.finished_at, before.finished_at);

    TournamentSlot::set_deadline_for_slots(
        fixture.tournament_id,
        &[fixture.slot_id],
        None,
        &mut conn,
    )
    .await
    .expect("clear scheduling deadline");
    assert!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .expect("load cleared scheduling deadline")
            .deadline_at
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawal_cancellation_is_rejected_outside_elimination() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("cancel_fmt", &mut conn).await;
    let result = diesel::update(tournament_slots::table.find(fixture.slot_id))
        .set((
            tournament_slots::resolution
                .eq(json!({"kind": "cancelled_by_withdrawal", "outcome": null})),
            tournament_slots::resolved_at.eq(Some(Utc::now())),
        ))
        .execute(&mut conn)
        .await;
    let Err(DieselError::DatabaseError(DatabaseErrorKind::CheckViolation, error)) = result else {
        panic!("Round Robin must reject elimination cancellations");
    };
    assert_eq!(
        error.constraint_name(),
        Some("tournament_slots_withdrawal_cancellation_format_check")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn slot_owned_games_are_bound_to_the_slots_exact_player_colors() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let fixture = started_schedule_fixture("slot_game_identity", &mut conn).await;
    let slot = TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
        .await
        .expect("load Slot-owned game identity");
    let game_id = games::table
        .filter(games::tournament_id.eq(fixture.tournament_id))
        .filter(games::tournament_slot_id.eq(fixture.slot_id))
        .select(games::id)
        .get_result::<Uuid>(&mut conn)
        .await
        .expect("load Slot-owned game");

    let swapped_colors = diesel::update(games::table.find(game_id))
        .set((
            games::white_id.eq(slot.black),
            games::black_id.eq(slot.white),
        ))
        .execute(&mut conn)
        .await;
    assert!(matches!(
        swapped_colors,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation,
            _
        ))
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn arena_configuration_persists_and_feature_ownership_is_enforced() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("arena_integrity_org", &mut conn).await;
    let players = [
        create_user("arena_int_first", &mut conn).await.id,
        create_user("arena_int_second", &mut conn).await.id,
    ];
    let configuration = arena_configuration();
    let arena_tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "Arena persistence integrity",
        TournamentStatus::NotStarted,
        Some(Utc::now() + Duration::hours(1)),
        configuration.clone(),
        &mut conn,
    )
    .await;
    let persisted_configuration = Tournament::find(arena_tournament.id, &mut conn).await;
    assert_eq!(
        persisted_configuration
            .expect("reload persisted Arena")
            .configuration(),
        &configuration,
    );

    let fixed = create_rr_tournament(
        organizer.id,
        "Fixed feature integrity",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let other_arena = create_rr_tournament_with_configuration(
        organizer.id,
        "Other Arena feature integrity",
        TournamentStatus::NotStarted,
        Some(Utc::now() + Duration::hours(1)),
        arena_configuration(),
        &mut conn,
    )
    .await;

    common::tournament::insert_unfrozen_memberships(
        arena_tournament.id,
        &players,
        fixed_instant(),
        &mut conn,
    )
    .await;
    diesel::update(tournaments::table.find(arena_tournament.id))
        .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::seconds(1))))
        .execute(&mut conn)
        .await
        .expect("make integrity Arena due");
    let present = players.into_iter().collect::<HashSet<_>>();
    let started = arena::start_scheduled_with_presence(
        arena_tournament.id,
        move |_| present.clone(),
        &mut conn,
    )
    .await
    .expect("start integrity Arena");
    let featured = started.new_games[0].id;
    assert_eq!(started.tournament.featured_game_id, Some(featured));

    let non_arena_feature = diesel::update(tournaments::table.find(fixed.id))
        .set(tournaments::featured_game_id.eq(Some(featured)))
        .execute(&mut conn)
        .await;
    assert!(matches!(
        non_arena_feature,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            _
        ))
    ));
    let cross_arena_feature = diesel::update(tournaments::table.find(other_arena.id))
        .set(tournaments::featured_game_id.eq(Some(featured)))
        .execute(&mut conn)
        .await;
    assert!(matches!(
        cross_arena_feature,
        Err(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation,
            _
        ))
    ));

    delete(games::table.find(featured))
        .execute(&mut conn)
        .await
        .expect("delete featured game");
    assert_eq!(
        Tournament::find(arena_tournament.id, &mut conn)
            .await
            .expect("reload Arena after featured-game deletion")
            .featured_game_id,
        None
    );
}

fn tournament_details(name: &str) -> TournamentDetails {
    TournamentDetails {
        name: name.to_string(),
        description: Some(String::from(
            "A deterministic canonical tournament fixture with enough detail for persistence tests.",
        )),
        seats: Some(4),
        min_seats: 2,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: None,
        configuration: round_robin_configuration(),
    }
}

fn arena_configuration() -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Arena(ArenaConfig::new(
            NonZeroU32::new(3_600).unwrap(),
            RealtimeClock {
                base_seconds: NonZeroU32::new(180).unwrap(),
                increment_seconds: 1,
            },
        )),
    }
}

async fn insert_membership(membership: &TournamentUser, conn: &mut DbConn<'_>) {
    diesel::insert_into(tournaments_users::table)
        .values(membership)
        .execute(conn)
        .await
        .expect("insert tournament membership fixture");
}

struct ScheduleFixture {
    organizer_id: Uuid,
    tournament_id: Uuid,
    white_id: Uuid,
    black_id: Uuid,
    slot_id: Uuid,
}

async fn started_schedule_fixture(prefix: &str, conn: &mut DbConn<'_>) -> ScheduleFixture {
    started_schedule_fixture_with_configuration(prefix, round_robin_configuration(), conn).await
}

async fn started_schedule_fixture_with_configuration(
    prefix: &str,
    configuration: Config,
    conn: &mut DbConn<'_>,
) -> ScheduleFixture {
    let organizer = create_user(&format!("{prefix}_o"), conn).await;
    let white = create_user(&format!("{prefix}_w"), conn).await;
    let black = create_user(&format!("{prefix}_b"), conn).await;
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        &format!("{prefix} persistence tournament"),
        TournamentStatus::NotStarted,
        None,
        configuration,
        conn,
    )
    .await;
    common::tournament::insert_unfrozen_memberships(
        tournament.id,
        &[white.id, black.id],
        fixed_instant(),
        conn,
    )
    .await;
    let started = fixed_field::start_by_organizer(
        tournament.id,
        organizer.id,
        vec![white.id, black.id],
        conn,
    )
    .await
    .expect("start scheduling fixture");
    let slot_id = started
        .games
        .as_slice()
        .first()
        .and_then(|game| game.tournament_slot_id)
        .expect("scheduling fixture releases a Slot-owned game");
    ScheduleFixture {
        organizer_id: organizer.id,
        tournament_id: tournament.id,
        white_id: white.id,
        black_id: black.id,
        slot_id,
    }
}

fn future_instant(minutes: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_micros((Utc::now() + Duration::minutes(minutes)).timestamp_micros())
        .expect("future scheduling fixture timestamp")
}

async fn insert_pending_offer(
    tournament_id: Uuid,
    slot_id: Uuid,
    proposer_id: Uuid,
    candidate_times: Vec<Option<DateTime<Utc>>>,
    conn: &mut DbConn<'_>,
) -> Result<Uuid, DieselError> {
    diesel::insert_into(schedule_offers::table)
        .values((
            schedule_offers::tournament_id.eq(tournament_id),
            schedule_offers::tournament_slot_id.eq(slot_id),
            schedule_offers::proposer_id.eq(proposer_id),
            schedule_offers::candidate_times.eq(candidate_times),
        ))
        .returning(schedule_offers::id)
        .get_result(conn)
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn scheduling_rejects_started_games_and_cancellation_preserves_newer_agreements() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let fixture = started_schedule_fixture("sched_cancel", &mut conn).await;
    let first = future_instant(60);
    let second = future_instant(120);
    let offer = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![first, second],
        &mut conn,
    )
    .await
    .unwrap()
    .pop()
    .unwrap();
    ScheduleOffer::accept(offer.id, fixture.black_id, first, &mut conn)
        .await
        .unwrap();
    let replacement = ScheduleOffer::propose(
        fixture.black_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![first],
        &mut conn,
    )
    .await
    .unwrap()
    .pop()
    .unwrap();
    ScheduleOffer::accept(replacement.id, fixture.white_id, first, &mut conn)
        .await
        .unwrap();
    let outsider = create_user("sched_outsider", &mut conn).await;
    assert!(matches!(
        ScheduleOffer::cancel_agreement(
            fixture.tournament_id,
            fixture.slot_id,
            outsider.id,
            first,
            &mut conn
        )
        .await,
        Err(DbError::Unauthorized)
    ));
    let changed = ScheduleOffer::cancel_agreement(
        fixture.tournament_id,
        fixture.slot_id,
        fixture.white_id,
        first,
        &mut conn,
    )
    .await
    .unwrap();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].id, replacement.id);
    let history = ScheduleOffer::find_for_tournament(fixture.tournament_id, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        history
            .iter()
            .find(|row| row.id == offer.id)
            .unwrap()
            .status()
            .unwrap(),
        ScheduleOfferStatus::Accepted
    );
    assert_eq!(changed[0].status().unwrap(), ScheduleOfferStatus::Cancelled);
    assert!(changed[0].selected_time.is_none());
    assert!(!changed[0].notified);
    assert!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .unwrap()
            .scheduled_at
            .is_none()
    );
    let pending = ScheduleOffer::propose(
        fixture.white_id,
        fixture.tournament_id,
        fixture.slot_id,
        vec![second],
        &mut conn,
    )
    .await
    .unwrap()
    .pop()
    .unwrap();
    ScheduleOffer::accept(pending.id, fixture.black_id, second, &mut conn)
        .await
        .unwrap();
    assert!(matches!(
        ScheduleOffer::cancel_agreement(
            fixture.tournament_id,
            fixture.slot_id,
            fixture.black_id,
            first,
            &mut conn
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
    assert_eq!(
        TournamentSlot::find(fixture.tournament_id, fixture.slot_id, &mut conn)
            .await
            .unwrap()
            .scheduled_at,
        Some(second)
    );
    let game_id = games::table
        .filter(games::tournament_slot_id.eq(Some(fixture.slot_id)))
        .select(games::id)
        .first::<Uuid>(&mut conn)
        .await
        .unwrap();
    db_lib::game_command::execute(
        game_id,
        db_lib::game_command::Command::StartReady {
            user_id: fixture.white_id,
        },
        &mut conn,
    )
    .await
    .unwrap();
    assert!(matches!(
        ScheduleOffer::propose(
            fixture.white_id,
            fixture.tournament_id,
            fixture.slot_id,
            vec![future_instant(180)],
            &mut conn
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
    // Acceptance shares the same authoritative check even if a pending offer remains.
    let pending = diesel::update(schedule_offers::table.find(pending.id))
        .set((
            schedule_offers::status.eq(ScheduleOfferStatus::Pending.as_str()),
            schedule_offers::selected_time.eq(None::<DateTime<Utc>>),
            schedule_offers::resolved_at.eq(None::<DateTime<Utc>>),
            schedule_offers::resolved_by.eq(None::<Uuid>),
        ))
        .get_result::<ScheduleOffer>(&mut conn)
        .await
        .unwrap();
    assert!(matches!(
        ScheduleOffer::accept(pending.id, fixture.black_id, second, &mut conn).await,
        Err(DbError::InvalidAction { .. })
    ));
    assert!(matches!(
        ScheduleOffer::cancel_agreement(
            fixture.tournament_id,
            fixture.slot_id,
            fixture.white_id,
            second,
            &mut conn
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
}
