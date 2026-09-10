use super::{
    evaluate_capabilities,
    fixed_field::{reconcile_fixed_field_at, StartOutcome},
    load_in_progress_for_update,
    persist_finished_with_outcome,
    project_round_robin_facts,
    state::BotUsers,
    FormatFacts,
};
use crate::{
    db_error::DbError,
    game_command::{execute, execute_at, Command, Outcome},
    get_conn,
    models::{
        Game,
        NewTournament,
        ScheduleOffer,
        Tournament,
        TournamentFinalOutcome,
        TournamentInvitation,
        TournamentSlot,
        TournamentSwissRound,
        TournamentUser,
    },
    schema::{
        game_hashes,
        games,
        ratings,
        schedule_offers,
        tournament_final_outcomes,
        tournaments,
        tournaments_invitations,
        users,
    },
    test_support::{
        db::test_db,
        tournament::{
            create_rr_tournament_with_configuration,
            create_user,
            fixed_instant,
            insert_unfrozen_memberships,
            realtime_clock,
            round_robin_configuration_with,
            round_robin_snapshot,
        },
    },
    tournaments::{arena as arena_api, fixed_field as fixed_field_db, public::load_by_id},
    DbConn,
};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{Color, Direction, GameControl, GameStatus, Position, Turn};
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        round_robin::{Criterion as RoundRobinCriterion, PrimaryScore as RoundRobinPrimaryScore},
        standings::Value as StandingValue,
        swiss::{
            Config as SwissConfig,
            PrimaryScore as DoubleSwissPrimaryScore,
            RoundConfiguration as SwissRoundConfiguration,
        },
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        BotAdmission,
        Config,
        Format,
        FormatConfig,
        GameOutcome,
        PlayedGameOutcome,
        RealtimeClock,
        ReleasePolicy,
        Resolution,
        Score,
    },
    tournament_view::TournamentFormatResponse,
    Conclusion,
    GameStart,
    ScheduleOfferStatus,
    TournamentDetails,
    TournamentGameResult,
    TournamentStatus,
};
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
};
use uuid::Uuid;

mod fixed_field_api {
    use super::*;

    pub async fn start_by_organizer(
        tournament_id: Uuid,
        organizer_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<StartOutcome, DbError> {
        let expected = TournamentUser::find_by_tournament_id(tournament_id, conn)
            .await?
            .into_iter()
            .map(|membership| membership.user_id)
            .collect();
        fixed_field_db::start_by_organizer(tournament_id, organizer_id, expected, conn).await
    }

    pub async fn adjudicate_slot_atomic(
        tournament_id: Uuid,
        slot_id: Uuid,
        result: TournamentGameResult,
        actor_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<fixed_field_db::AdjudicationOutcome, DbError> {
        let expected = TournamentSlot::find(tournament_id, slot_id, conn)
            .await?
            .resolution;
        fixed_field_db::adjudicate_slot_atomic(
            tournament_id,
            slot_id,
            result,
            expected,
            actor_id,
            conn,
        )
        .await
    }

    pub async fn close_unstarted_slots(
        tournament_id: Uuid,
        actor_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<fixed_field_db::CloseoutOutcome, DbError> {
        let tournament = Tournament::find(tournament_id, conn).await?;
        let slot_ids = if tournament.status() == TournamentStatus::InProgress {
            let state =
                super::super::state::load_in_progress(tournament, BotUsers::ForGameRelease, conn)
                    .await?;
            let facts = match state.configuration.format() {
                Format::RoundRobin => {
                    let projected = project_round_robin_facts(&state)?;
                    evaluate_capabilities(&state, FormatFacts::RoundRobin(&projected))?
                }
                Format::Swiss | Format::DoubleSwiss => {
                    evaluate_capabilities(&state, FormatFacts::Swiss)?
                }
                _ => {
                    return Err(DbError::InvalidAction {
                        info: String::from("Unsupported closeout format"),
                    })
                }
            };
            facts.closeout_slot_ids.unwrap_or_default()
        } else {
            Vec::new()
        };
        fixed_field_db::close_unstarted_slots(tournament_id, actor_id, slot_ids, conn).await
    }
}

struct RoundRobinScenario {
    tournament: Tournament,
    organizer_id: Uuid,
    players: [Uuid; 2],
}

impl RoundRobinScenario {
    async fn unstarted(
        prefix: &str,
        repeats: NonZeroU32,
        release_policy: ReleasePolicy,
        conn: &mut DbConn<'_>,
    ) -> Self {
        let organizer = create_user(&format!("{prefix}_o"), conn).await;
        let players = [
            create_user(&format!("{prefix}_p0"), conn).await.id,
            create_user(&format!("{prefix}_p1"), conn).await.id,
        ];
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            prefix,
            TournamentStatus::NotStarted,
            None,
            round_robin_configuration_with(repeats, release_policy),
            conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), conn).await;
        Self {
            tournament,
            organizer_id: organizer.id,
            players,
        }
    }

    async fn start(&self, conn: &mut DbConn<'_>) -> StartOutcome {
        fixed_field_api::start_by_organizer(self.tournament.id, self.organizer_id, conn)
            .await
            .expect("start Round Robin scenario")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn round_robin_match_awards_reach_live_projection_and_persisted_final_standings() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("rr_match_o", &mut conn).await;
    let players = [
        create_user("rr_match_a", &mut conn).await.id,
        create_user("rr_match_b", &mut conn).await.id,
    ];
    let mut configuration =
        round_robin_configuration_with(NonZeroU32::new(4).unwrap(), ReleasePolicy::FullyUnlocked);
    let FormatConfig::RoundRobin(config) = &mut configuration.format else {
        unreachable!()
    };
    config.primary_score = RoundRobinPrimaryScore::MatchPoints;
    config.standings.push(RoundRobinCriterion::GamePoints);
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "rr_match_projection",
        TournamentStatus::NotStarted,
        None,
        configuration,
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .unwrap();
    let slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .unwrap();
    assert_eq!(slots.len(), 4);
    for (index, slot) in slots.iter().enumerate() {
        let winner = if slot.white == players[0] {
            Color::White
        } else {
            Color::Black
        };
        fixed_field_api::adjudicate_slot_atomic(
            tournament.id,
            slot.id,
            TournamentGameResult::Winner(winner),
            organizer.id,
            &mut conn,
        )
        .await
        .unwrap();
        let snapshot = load_by_id(tournament.id, &mut conn).await.unwrap();
        let complete = index == 3;
        let TournamentFormatResponse::RoundRobin { matches, .. } = snapshot.format else {
            unreachable!()
        };
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].completion.is_some(), complete);
        let standings = snapshot.standings.expect("projected standings");
        let winner = standings
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.user_id == players[0])
            .unwrap();
        assert_eq!(
            winner.primary_score,
            StandingValue::Score(Score::new(if complete { 2 } else { 0 }))
        );
        assert_eq!(
            winner.values[1],
            StandingValue::Score(Score::new((index as u32 + 1) * 2))
        );
        let persisted = TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .unwrap();
        if complete {
            assert_eq!(persisted.unwrap().standings, standings);
            assert_eq!(winner.counts[1], 1);
        } else {
            assert!(persisted.is_none());
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_transitions_preserve_idempotence_capacity_and_declined_admission() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("inv_contract_o", &mut conn).await;
    let pending = create_user("inv_pending", &mut conn).await;
    let public_joiner = create_user("inv_public", &mut conn).await;
    let fillers = [
        create_user("inv_f0", &mut conn).await.id,
        create_user("inv_f1", &mut conn).await.id,
        create_user("inv_f2", &mut conn).await.id,
    ];
    let full_candidate = create_user("inv_full", &mut conn).await;
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "invitation_transition_contract",
        TournamentStatus::NotStarted,
        None,
        round_robin_configuration_with(NonZeroU32::new(1).unwrap(), ReleasePolicy::FullyUnlocked),
        &mut conn,
    )
    .await;

    let created = tournament
        .create_invitation(&organizer.id, &pending.id, &mut conn)
        .await
        .expect("create pending invitation");
    assert!(created.changed);
    let duplicate = tournament
        .create_invitation(&organizer.id, &pending.id, &mut conn)
        .await
        .expect("repeat pending invitation idempotently");
    assert!(!duplicate.changed);
    assert_eq!(invitation_count(tournament.id, &mut conn).await, 1);

    tournament
        .decline_invitation(&pending.id, &mut conn)
        .await
        .expect("decline pending invitation");
    assert!(
        TournamentInvitation::find_by_ids(&tournament.id, &pending.id, &mut conn)
            .await
            .expect("load declined invitation")
            .is_some_and(|invitation| invitation.declined_at.is_some())
    );
    let reinvited = tournament
        .create_invitation(&organizer.id, &pending.id, &mut conn)
        .await
        .expect("reopen declined invitation");
    assert!(reinvited.changed);
    assert!(
        TournamentInvitation::find_active_by_ids(&tournament.id, &pending.id, &mut conn)
            .await
            .expect("load reopened invitation")
            .is_some()
    );

    tournament
        .create_invitation(&organizer.id, &public_joiner.id, &mut conn)
        .await
        .expect("invite public joiner");
    tournament
        .decline_invitation(&public_joiner.id, &mut conn)
        .await
        .expect("decline before public join");
    tournament
        .join(&public_joiner.id, &mut conn)
        .await
        .expect("join public tournament after declining");
    assert!(
        TournamentInvitation::find_by_ids(&tournament.id, &public_joiner.id, &mut conn)
            .await
            .expect("check consumed declined invitation")
            .is_none()
    );
    assert!(
        TournamentUser::contains(tournament.id, public_joiner.id, &mut conn)
            .await
            .expect("check public membership")
    );

    let member = tournament
        .create_invitation(&organizer.id, &public_joiner.id, &mut conn)
        .await
        .expect("ignore existing member invitation");
    assert!(!member.changed);
    let organizer_invite = tournament
        .create_invitation(&organizer.id, &organizer.id, &mut conn)
        .await
        .expect("invite organizer to participate as a player");
    assert!(organizer_invite.changed);

    insert_unfrozen_memberships(tournament.id, &fillers, fixed_instant(), &mut conn).await;
    let pending_while_full = tournament
        .create_invitation(&organizer.id, &pending.id, &mut conn)
        .await
        .expect("keep pending retry idempotent after field fills");
    assert!(!pending_while_full.changed);
    assert!(matches!(
        tournament
            .create_invitation(&organizer.id, &full_candidate.id, &mut conn)
            .await,
        Err(DbError::TournamentFull)
    ));

    let invite_only = create_rr_tournament_with_configuration(
        organizer.id,
        "declined_invite_only_contract",
        TournamentStatus::NotStarted,
        None,
        round_robin_configuration_with(NonZeroU32::new(1).unwrap(), ReleasePolicy::FullyUnlocked),
        &mut conn,
    )
    .await;
    let invite_only = diesel::update(tournaments::table.find(invite_only.id))
        .set(tournaments::invite_only.eq(true))
        .get_result::<Tournament>(&mut conn)
        .await
        .expect("make fixture invite-only");
    invite_only
        .create_invitation(&organizer.id, &full_candidate.id, &mut conn)
        .await
        .expect("create invite-only invitation");
    invite_only
        .decline_invitation(&full_candidate.id, &mut conn)
        .await
        .expect("decline invite-only invitation");
    assert!(matches!(
        invite_only.join(&full_candidate.id, &mut conn).await,
        Err(DbError::TournamentInviteOnly)
    ));
    assert!(
        TournamentInvitation::find_by_ids(&invite_only.id, &full_candidate.id, &mut conn)
            .await
            .expect("load preserved declined invite-only row")
            .is_some_and(|invitation| invitation.declined_at.is_some())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_is_atomic_and_retryable() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "start_atomic",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let invitee = create_user("start_atomic_invitee", &mut conn).await;
    TournamentInvitation::new(scenario.tournament.id, invitee.id)
        .insert(&mut conn)
        .await
        .expect("insert outstanding invitation");

    let tournament_id = scenario.tournament.id;
    let organizer_id = scenario.organizer_id;
    let rolled_back = conn
        .transaction::<_, DbError, _>(async move |tc| {
            fixed_field_api::start_by_organizer(tournament_id, organizer_id, tc).await?;
            Err::<(), DbError>(DbError::InvalidAction {
                info: String::from("force outer transaction rollback"),
            })
        })
        .await;
    assert!(matches!(rolled_back, Err(DbError::InvalidAction { .. })));
    assert_start_rollback_restored_state(tournament_id, &mut conn).await;
    assert_eq!(invitation_count(tournament_id, &mut conn).await, 1);

    let started = scenario.start(&mut conn).await;
    assert!(started.started_now);
    assert_eq!(started.tournament.status(), TournamentStatus::InProgress);
    assert_eq!(started.games.len(), 1);
    assert_eq!(started.removed_invitees, vec![invitee.id]);
    assert!(
        TournamentUser::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .expect("reload frozen memberships")
            .iter()
            .all(|membership| membership.pairing_number.is_some())
    );
    assert_eq!(invitation_count(tournament_id, &mut conn).await, 0);

    let duplicate = fixed_field_api::start_by_organizer(
        scenario.tournament.id,
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect_err("duplicate start is rejected");
    assert!(matches!(duplicate, DbError::InvalidAction { .. }));
    assert_eq!(
        TournamentSlot::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .expect("reload slots after duplicate start")
            .len(),
        1
    );
    assert_eq!(
        scenario
            .tournament
            .games(&mut conn)
            .await
            .expect("reload games")
            .len(),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn capability_projection_is_objective_and_scoped_to_lifecycle() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "capability_scope",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    scenario.start(&mut conn).await;

    let mut state =
        load_in_progress_for_update(scenario.tournament.id, &[], BotUsers::Skip, &mut conn)
            .await
            .expect("load capability state");
    let projected = project_round_robin_facts(&state).expect("project Round Robin facts");
    let facts = FormatFacts::RoundRobin(&projected);

    let entrant_capabilities =
        evaluate_capabilities(&state, facts).expect("project objective capabilities");
    assert_eq!(
        entrant_capabilities.withdrawable_entrants,
        scenario.players.into_iter().collect()
    );
    assert!(entrant_capabilities.closeout_eligible_slots.is_some());
    assert!(entrant_capabilities
        .slots
        .values()
        .all(|slot| !slot.admin_actions.is_empty()));

    state.tournament.finished_at = Some(Utc::now());
    let finished_capabilities =
        evaluate_capabilities(&state, facts).expect("project finished capabilities");
    assert!(finished_capabilities.withdrawable_entrants.is_empty());
    assert!(finished_capabilities.closeout_eligible_slots.is_none());
    assert!(finished_capabilities
        .slots
        .values()
        .all(|slot| slot.admin_actions.is_empty()));
}

#[tokio::test(flavor = "multi_thread")]
async fn round_robin_withdrawal_reports_prior_adjudicated_slots_for_capability_refresh() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let prefix = "rr_withdraw_patch";
    let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
    let players = [
        create_user(&format!("{prefix}_p0"), &mut conn).await.id,
        create_user(&format!("{prefix}_p1"), &mut conn).await.id,
        create_user(&format!("{prefix}_p2"), &mut conn).await.id,
        create_user(&format!("{prefix}_p3"), &mut conn).await.id,
    ];
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        prefix,
        TournamentStatus::NotStarted,
        None,
        round_robin_configuration_with(NonZeroU32::new(1).unwrap(), ReleasePolicy::FullyUnlocked),
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Round Robin withdrawal patch scenario");
    let slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load Round Robin slots");
    let withdrawn_player = players[0];
    let prior_adjudicated = slots
        .iter()
        .find(|slot| slot.white == withdrawn_player || slot.black == withdrawn_player)
        .expect("withdrawing entrant has a slot");
    fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        prior_adjudicated.id,
        TournamentGameResult::Winner(Color::White),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("adjudicate one prior result");

    let withdrawal =
        fixed_field_db::withdraw_player(tournament.id, withdrawn_player, organizer.id, &mut conn)
            .await
            .expect("withdraw Round Robin entrant");
    let commit = withdrawal.commit.expect("withdrawal reports its changes");
    assert!(commit.availability_changed);

    assert!(!commit.finished_now);
    assert!(commit.affected_slot_ids.contains(&prior_adjudicated.id));
}

#[tokio::test(flavor = "multi_thread")]
async fn finishing_round_robin_reports_every_slot_for_capability_refresh() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let prefix = "rr_finish_patch";
    let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
    let players = [
        create_user(&format!("{prefix}_p0"), &mut conn).await.id,
        create_user(&format!("{prefix}_p1"), &mut conn).await.id,
        create_user(&format!("{prefix}_p2"), &mut conn).await.id,
        create_user(&format!("{prefix}_p3"), &mut conn).await.id,
    ];
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        prefix,
        TournamentStatus::NotStarted,
        None,
        round_robin_configuration_with(NonZeroU32::new(1).unwrap(), ReleasePolicy::FullyUnlocked),
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Round Robin finish patch scenario");
    let slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load Round Robin slots");
    let mut final_commit = None;
    for slot in &slots {
        let outcome = fixed_field_api::adjudicate_slot_atomic(
            tournament.id,
            slot.id,
            TournamentGameResult::Winner(Color::White),
            organizer.id,
            &mut conn,
        )
        .await
        .expect("adjudicate Round Robin result");
        if outcome
            .commit
            .as_ref()
            .is_some_and(|commit| commit.finished_now)
        {
            final_commit = outcome.commit;
        }
    }
    let commit = final_commit.expect("final result finishes the tournament");
    let mut expected_slot_ids = slots.iter().map(|slot| slot.id).collect::<Vec<_>>();
    expected_slot_ids.sort_unstable();

    assert!(commit.availability_changed);
    assert_eq!(commit.affected_slot_ids, expected_slot_ids);
}

#[tokio::test(flavor = "multi_thread")]
async fn sequential_round_robin_waits_for_its_unresolved_predecessor() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "rr_block",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let started = scenario.start(&mut conn).await;
    assert_eq!(started.games.len(), 1);
    let predecessor_slot_id = slot_id_for_game(&started.games[0]);

    let successor_slot_id = {
        let state =
            load_in_progress_for_update(scenario.tournament.id, &[], BotUsers::Skip, &mut conn)
                .await
                .expect("load sequential release state");
        let successor = state
            .slots
            .iter()
            .find(|slot| slot.id != predecessor_slot_id)
            .expect("persist the planned repeat");
        assert!(state
            .games
            .iter()
            .all(|game| game.tournament_slot_id != Some(successor.id)));
        let projected = project_round_robin_facts(&state).expect("project Round Robin facts");
        let capabilities = evaluate_capabilities(&state, FormatFacts::RoundRobin(&projected))
            .expect("project sequential release capabilities");
        let successor_capabilities = capabilities
            .slots
            .get(&successor.id)
            .expect("project the planned repeat");
        assert_eq!(successor_capabilities.waits_for, Some(predecessor_slot_id));
        successor.id
    };

    fixed_field_api::adjudicate_slot_atomic(
        scenario.tournament.id,
        predecessor_slot_id,
        TournamentGameResult::Winner(Color::White),
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("resolve the direct release predecessor");

    let state = load_in_progress_for_update(scenario.tournament.id, &[], BotUsers::Skip, &mut conn)
        .await
        .expect("reload released repeat");
    assert!(state
        .games
        .iter()
        .any(|game| game.tournament_slot_id == Some(successor_slot_id)));
    let projected = project_round_robin_facts(&state).expect("reproject Round Robin facts");
    let capabilities = evaluate_capabilities(&state, FormatFacts::RoundRobin(&projected))
        .expect("reproject released repeat capabilities");
    assert_eq!(
        capabilities
            .slots
            .get(&successor_slot_id)
            .expect("project released repeat")
            .waits_for,
        None
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn scheduled_start_candidates_order_limit_and_dispatch_atomically() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let as_of = DateTime::from_timestamp_micros(Utc::now().timestamp_micros())
        .expect("scheduled observation timestamp");

    let organizer = create_user("sched_o", &mut conn).await;
    let players = [
        create_user("sched_p0", &mut conn).await.id,
        create_user("sched_p1", &mut conn).await.id,
    ];
    let scheduled_rr = || {
        round_robin_configuration_with(
            NonZeroU32::new(1).unwrap(),
            ReleasePolicy::SequentialPerMatchup,
        )
    };
    let undersubscribed = create_rr_tournament_with_configuration(
        organizer.id,
        "scheduled_contract_early",
        TournamentStatus::NotStarted,
        Some(as_of + Duration::hours(1)),
        scheduled_rr(),
        &mut conn,
    )
    .await;
    let undersubscribed_due = as_of - Duration::seconds(20);
    diesel::update(tournaments::table.find(undersubscribed.id))
        .set(tournaments::starts_at.eq(Some(undersubscribed_due)))
        .execute(&mut conn)
        .await
        .expect("make undersubscribed tournament due");
    insert_unfrozen_memberships(
        undersubscribed.id,
        &players[..1],
        fixed_instant(),
        &mut conn,
    )
    .await;
    let ready = create_rr_tournament_with_configuration(
        organizer.id,
        "scheduled_contract_ready",
        TournamentStatus::NotStarted,
        Some(as_of + Duration::hours(1)),
        scheduled_rr(),
        &mut conn,
    )
    .await;
    let organizer_trigger_for_scheduled =
        fixed_field_api::start_by_organizer(ready.id, organizer.id, &mut conn)
            .await
            .expect_err("organizer cannot trigger a scheduled tournament");
    assert!(matches!(
        organizer_trigger_for_scheduled,
        DbError::InvalidAction { .. }
    ));
    let scheduled_before_due = fixed_field_db::start_scheduled(ready.id, &mut conn)
        .await
        .expect_err("scheduler cannot trigger a future tournament");
    assert!(matches!(
        scheduled_before_due,
        DbError::InvalidAction { .. }
    ));

    let organizer_only = create_rr_tournament_with_configuration(
        organizer.id,
        "scheduled_contract_organizer",
        TournamentStatus::NotStarted,
        None,
        scheduled_rr(),
        &mut conn,
    )
    .await;
    let scheduled_trigger_for_organizer =
        fixed_field_db::start_scheduled(organizer_only.id, &mut conn)
            .await
            .expect_err("scheduler cannot trigger an organizer-start tournament");
    assert!(matches!(
        scheduled_trigger_for_organizer,
        DbError::InvalidAction { .. }
    ));

    let ready_due = as_of - Duration::seconds(10);
    diesel::update(tournaments::table.find(ready.id))
        .set(tournaments::starts_at.eq(Some(ready_due)))
        .execute(&mut conn)
        .await
        .expect("make ready tournament due");
    insert_unfrozen_memberships(ready.id, &players, fixed_instant(), &mut conn).await;

    assert_eq!(
        fixed_field_db::scheduled_start_candidates(as_of, 1, &mut conn)
            .await
            .expect("load bounded fixed-field candidates"),
        vec![undersubscribed.id],
    );
    assert_eq!(
        fixed_field_db::scheduled_start_candidates(as_of, 0, &mut conn)
            .await
            .expect("zero limit is empty"),
        Vec::<Uuid>::new(),
    );
    let fallback = fixed_field_db::start_scheduled(undersubscribed.id, &mut conn)
        .await
        .expect("apply undersubscribed fallback");
    assert!(!fallback.started_now);
    assert_eq!(fallback.tournament.status(), TournamentStatus::NotStarted);
    assert!(fallback.tournament.starts_at.is_none());
    assert_eq!(fallback.tournament.configuration(), &scheduled_rr());
    assert!(fallback.tournament.started_at.is_none());
    assert!(fallback.tournament.finished_at.is_none());
    assert!(fallback.games.is_empty());
    assert!(undersubscribed
        .games(&mut conn)
        .await
        .expect("reload undersubscribed games")
        .is_empty());
    assert!(
        TournamentSlot::find_by_tournament_id(undersubscribed.id, &mut conn)
            .await
            .expect("reload undersubscribed slots")
            .is_empty()
    );
    insert_unfrozen_memberships(
        undersubscribed.id,
        &players[1..],
        fixed_instant(),
        &mut conn,
    )
    .await;
    let organizer_started =
        fixed_field_api::start_by_organizer(undersubscribed.id, organizer.id, &mut conn)
            .await
            .expect("organizer can start after the fallback and sufficient signup");
    assert!(organizer_started.started_now);
    assert_eq!(organizer_started.games.len(), 1);

    let started = fixed_field_db::start_scheduled(ready.id, &mut conn)
        .await
        .expect("start due fixed-field tournament");
    assert!(started.started_now);
    assert_eq!(started.tournament.status(), TournamentStatus::InProgress);
    assert_eq!(started.games.len(), 1);

    let arena = create_rr_tournament_with_configuration(
        organizer.id,
        "scheduled_contract_arena",
        TournamentStatus::NotStarted,
        Some(as_of + Duration::hours(1)),
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
    let arena_due = as_of - Duration::seconds(5);
    diesel::update(tournaments::table.find(arena.id))
        .set(tournaments::starts_at.eq(Some(arena_due)))
        .execute(&mut conn)
        .await
        .expect("make Arena tournament due");
    insert_unfrozen_memberships(arena.id, &players, fixed_instant(), &mut conn).await;
    assert_eq!(
        arena_api::scheduled_start_candidates(as_of, 1, &mut conn)
            .await
            .expect("load Arena candidate"),
        vec![arena.id],
    );
    let online = move |_: &[Uuid]| players.into_iter().collect::<HashSet<_>>();
    let arena_started = arena_api::start_scheduled_with_presence(arena.id, online, &mut conn)
        .await
        .expect("start due Arena");
    assert!(arena_started.started_now);
    assert_eq!(
        arena_started.tournament.status(),
        TournamentStatus::InProgress
    );
    assert_eq!(arena_started.new_games.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn released_bot_game_starts_immediately() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "bot_ready",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    diesel::update(users::table.filter(users::id.eq_any(scenario.players)))
        .set(users::bot.eq(true))
        .execute(&mut conn)
        .await
        .expect("mark both entrants as bots");

    let game = scenario.start(&mut conn).await.games.remove(0);
    assert_eq!(game.game_start, GameStart::Ready.to_string());
    assert_eq!(game.game_status, GameStatus::InProgress.to_string());
    assert!(game.last_interaction.is_some());
    assert!(game.timeout_at.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn ready_start_rejects_a_resolved_slot_and_a_terminal_game() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    let resolved = RoundRobinScenario::unstarted(
        "ready_resolved",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let resolved_game = resolved.start(&mut conn).await.games.remove(0);
    let resolved_slot_id = slot_id_for_game(&resolved_game);
    let mut resolved_slot =
        TournamentSlot::find(resolved.tournament.id, resolved_slot_id, &mut conn)
            .await
            .expect("load Ready game's Slot");
    resolved_slot.resolution = Some(Resolution::Result(GameOutcome::Adjudicated(
        black_forfeit_win(),
    )));
    TournamentSlot::persist_resolution(
        resolved.tournament.id,
        &resolved_slot,
        Utc::now(),
        &mut conn,
    )
    .await
    .expect("seal fixture Slot");

    assert!(matches!(
        execute(
            resolved_game.id,
            Command::StartReady {
                user_id: resolved_game.white_id,
            },
            &mut conn,
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
    assert_eq!(
        Game::find_by_uuid(&resolved_game.id, &mut conn)
            .await
            .expect("reload rejected Ready game")
            .game_status,
        GameStatus::NotStarted.to_string(),
    );

    let terminal = RoundRobinScenario::unstarted(
        "ready_terminal",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let terminal_game = terminal.start(&mut conn).await.games.remove(0);
    terminal_game
        .adjudicate_unstarted(
            &TournamentGameResult::Winner(Color::White),
            Conclusion::Committee,
            Utc::now(),
            &mut conn,
        )
        .await
        .expect("terminalize fixture Game without resolving its Slot");

    assert!(matches!(
        execute(
            terminal_game.id,
            Command::StartReady {
                user_id: terminal_game.white_id,
            },
            &mut conn,
        )
        .await,
        Err(DbError::GameIsOver)
    ));
    assert!(TournamentSlot::find(
        terminal.tournament.id,
        slot_id_for_game(&terminal_game),
        &mut conn,
    )
    .await
    .expect("reload terminal Game's Slot")
    .resolution
    .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_schedule_proposals_are_hidden_and_cannot_be_accepted() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "expired_schedule",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let game = scenario.start(&mut conn).await.games.remove(0);
    let slot_id = slot_id_for_game(&game);
    let offer = ScheduleOffer::propose(
        game.white_id,
        scenario.tournament.id,
        slot_id,
        vec![Utc::now() + Duration::hours(1)],
        &mut conn,
    )
    .await
    .expect("insert schedule offer")
    .pop()
    .expect("inserted schedule offer is returned");
    let expired_time = Utc::now() - Duration::minutes(1);
    diesel::update(schedule_offers::table.find(offer.id))
        .set(schedule_offers::candidate_times.eq(vec![Some(expired_time)]))
        .execute(&mut conn)
        .await
        .expect("expire schedule offer");

    assert!(
        ScheduleOffer::find_user_notifications(game.black_id, &mut conn)
            .await
            .expect("load schedule offer notifications")
            .is_empty()
    );
    assert!(matches!(
        ScheduleOffer::accept(offer.id, game.black_id, expired_time, &mut conn).await,
        Err(DbError::InvalidAction { .. })
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_game_commits_while_tournament_is_locked_and_reconciliation_advances_once() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "game_progression",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let game = scenario.start(&mut conn).await.games.remove(0);
    let slot_id = slot_id_for_game(&game);
    execute(
        game.id,
        Command::StartReady {
            user_id: game.white_id,
        },
        &mut conn,
    )
    .await
    .expect("start Ready game");

    for (actor, piece, position) in [
        (game.white_id, "wA1", Position::initial_spawn_position()),
        (
            game.black_id,
            "bA1",
            Position::initial_spawn_position().to(Direction::E),
        ),
    ] {
        execute(
            game.id,
            Command::Move {
                user_id: actor,
                turn: Turn::Move(piece.parse().expect("parse ant"), position),
                compensation: 0.0,
            },
            &mut conn,
        )
        .await
        .expect("apply legal move");
    }
    execute(
        game.id,
        Command::Control {
            user_id: game.white_id,
            control: GameControl::DrawOffer(Color::White),
        },
        &mut conn,
    )
    .await
    .expect("offer draw");
    let tournament_id = scenario.tournament.id;
    let game_id = game.id;
    let black_id = game.black_id;
    let terminal_at = Utc::now();
    let command_pool = db.pool.clone();
    let terminal = conn
        .transaction::<_, DbError, _>(async move |tc| {
            Tournament::find_for_update(tournament_id, tc).await?;
            let command = tokio::spawn(async move {
                let mut command_conn = get_conn(&command_pool)
                    .await
                    .expect("get terminal command connection");
                execute_at(
                    game_id,
                    Command::Control {
                        user_id: black_id,
                        control: GameControl::DrawAccept(Color::Black),
                    },
                    terminal_at,
                    &mut command_conn,
                )
                .await
            });
            let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), command)
                .await
                .expect("terminal Game command does not wait for the Tournament lock")
                .expect("join terminal Game command")?;
            assert!(TournamentSlot::find(tournament_id, slot_id, tc)
                .await?
                .resolution
                .is_none());
            Ok(outcome)
        })
        .await
        .expect("commit terminal Game while Tournament is locked elsewhere");
    let Outcome::Applied {
        game: terminal,
        newly_terminal: true,
        ..
    } = terminal
    else {
        panic!("draw acceptance commits only the terminal Game")
    };
    assert_eq!(terminal.conclusion, Conclusion::Draw.to_string());
    let finished_at = terminal
        .finished_at
        .expect("terminal Game records finished_at");
    assert!(TournamentSlot::find(tournament_id, slot_id, &mut conn)
        .await
        .expect("load durable recovery marker")
        .resolution
        .is_none());

    let (offer_ids, selected_time) = create_accepted_offer_with_pending_reschedule(
        tournament_id,
        slot_id,
        game.white_id,
        game.black_id,
        &mut conn,
    )
    .await;
    assert_eq!(
        TournamentSlot::find(tournament_id, slot_id, &mut conn)
            .await
            .expect("load scheduled unresolved Slot")
            .scheduled_at,
        Some(selected_time),
    );
    assert_eq!(
        fixed_field_db::reconciliation_candidates(10, &mut conn)
            .await
            .expect("load reconciliation candidates")
            .into_iter()
            .filter(|candidate| *candidate == tournament_id)
            .count(),
        1
    );

    let reconciled_at = finished_at + Duration::minutes(5);
    let first_pool = db.pool.clone();
    let second_pool = db.pool.clone();
    let first = tokio::spawn(async move {
        let mut reconcile_conn = get_conn(&first_pool)
            .await
            .expect("get first reconciliation connection");
        reconcile_fixed_field_at(tournament_id, reconciled_at, &mut reconcile_conn).await
    });
    let second = tokio::spawn(async move {
        let mut reconcile_conn = get_conn(&second_pool)
            .await
            .expect("get second reconciliation connection");
        reconcile_fixed_field_at(tournament_id, reconciled_at, &mut reconcile_conn).await
    });
    let (first, second) = tokio::join!(first, second);
    let first = first
        .expect("join first reconciliation")
        .expect("first reconciliation succeeds");
    let second = second
        .expect("join second reconciliation")
        .expect("second reconciliation succeeds");
    assert_eq!(
        usize::from(first.is_some()) + usize::from(second.is_some()),
        1
    );
    let commit = first
        .as_ref()
        .or(second.as_ref())
        .expect("one reconciliation reports the exact committed changes");
    assert!(commit.standings_changed);
    assert!(!commit.format_changed);
    assert!(commit.catalog_changed);
    assert!(!commit.finished_now);

    let sealed = TournamentSlot::find(tournament_id, slot_id, &mut conn)
        .await
        .expect("load reconciled Slot");
    assert_eq!(sealed.resolved_at, Some(finished_at));
    assert!(sealed.scheduled_at.is_none());
    assert!(matches!(
        sealed.resolution,
        Some(Resolution::Result(GameOutcome::Played(
            PlayedGameOutcome::Draw
        )))
    ));
    assert_eq!(
        schedule_offers::table
            .filter(schedule_offers::id.eq_any(&offer_ids))
            .select(ScheduleOffer::as_select())
            .load::<ScheduleOffer>(&mut conn)
            .await
            .expect("load schedule history after reconciliation")
            .iter()
            .filter(|offer| offer.status().unwrap() == ScheduleOfferStatus::Pending)
            .count(),
        0
    );
    assert_eq!(
        game_hashes::table
            .filter(game_hashes::game_id.eq(game.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count persisted explorer rows"),
        2
    );
    let games = scenario
        .tournament
        .games(&mut conn)
        .await
        .expect("load games after reconciliation");
    assert_eq!(games.len(), 2);
    let successor = games
        .iter()
        .find(|candidate| candidate.id != game.id)
        .expect("reconciliation releases one successor");
    let mut expected_slot_ids = vec![slot_id, slot_id_for_game(successor)];
    expected_slot_ids.sort_unstable();
    assert_eq!(commit.affected_slot_ids, expected_slot_ids);
    assert_eq!(successor.created_at, reconciled_at);
    assert_eq!(
        fixed_field_db::reconciliation_candidates(10, &mut conn)
            .await
            .expect("reload reconciliation candidates")
            .into_iter()
            .filter(|candidate| *candidate == tournament_id)
            .count(),
        0
    );
    assert!(
        reconcile_fixed_field_at(tournament_id, reconciled_at, &mut conn)
            .await
            .expect("repeat reconciliation succeeds")
            .is_none()
    );
    assert_eq!(
        scenario
            .tournament
            .games(&mut conn)
            .await
            .expect("reload games after repeated reconciliation")
            .len(),
        2
    );
    assert!(matches!(
        ScheduleOffer::propose(
            game.white_id,
            tournament_id,
            slot_id,
            vec![Utc::now() + Duration::hours(1)],
            &mut conn,
        )
        .await,
        Err(DbError::InvalidAction { .. })
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn reconciliation_folds_every_terminal_marker_and_finishes_once() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "reconcile_all",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    let games = scenario.start(&mut conn).await.games;
    assert_eq!(games.len(), 2);

    let base_time = Utc::now();
    let mut finished_at_by_slot = HashMap::new();
    for (index, game) in games.iter().enumerate() {
        let started_at = base_time + Duration::seconds(index as i64 * 2);
        execute_at(
            game.id,
            Command::StartReady {
                user_id: game.white_id,
            },
            started_at,
            &mut conn,
        )
        .await
        .expect("start recovery fixture Game");
        let terminal = execute_at(
            game.id,
            Command::Control {
                user_id: game.white_id,
                control: GameControl::Resign(Color::White),
            },
            started_at + Duration::seconds(1),
            &mut conn,
        )
        .await
        .expect("commit recovery fixture result");
        let Outcome::Applied {
            game: terminal,
            newly_terminal: true,
            ..
        } = terminal
        else {
            panic!("fixed-field result commits before tournament reconciliation")
        };
        finished_at_by_slot.insert(
            slot_id_for_game(&terminal),
            terminal
                .finished_at
                .expect("terminal Game records finished_at"),
        );
    }

    assert_eq!(
        fixed_field_db::reconciliation_candidates(1, &mut conn)
            .await
            .expect("load bounded distinct recovery candidates"),
        vec![scenario.tournament.id]
    );
    let effects = reconcile_fixed_field_at(
        scenario.tournament.id,
        base_time + Duration::minutes(5),
        &mut conn,
    )
    .await
    .expect("reconcile every committed result")
    .expect("reconciliation changes tournament state");
    assert!(effects.finished_now);
    assert!(effects.released_game_ids.is_empty());
    assert!(effects.standings_changed);
    assert!(!effects.format_changed);
    assert!(effects.catalog_changed);
    let mut expected_slot_ids = finished_at_by_slot.keys().copied().collect::<Vec<_>>();
    expected_slot_ids.sort_unstable();
    assert_eq!(effects.affected_slot_ids, expected_slot_ids);

    let slots = TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("reload reconciled Slots");
    assert_eq!(slots.len(), 2);
    assert!(slots.iter().all(|slot| {
        slot.resolution.is_some() && slot.resolved_at == finished_at_by_slot.get(&slot.id).copied()
    }));
    assert_eq!(
        Tournament::find(scenario.tournament.id, &mut conn)
            .await
            .expect("reload automatically finished tournament")
            .status(),
        TournamentStatus::Finished
    );
    assert_eq!(
        final_outcome_count(scenario.tournament.id, &mut conn).await,
        1
    );
    assert!(reconcile_fixed_field_at(
        scenario.tournament.id,
        base_time + Duration::minutes(6),
        &mut conn,
    )
    .await
    .expect("repeat finished reconciliation succeeds")
    .is_none());
    assert_eq!(
        final_outcome_count(scenario.tournament.id, &mut conn).await,
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_adjudication_folds_another_committed_terminal_marker_before_finishing() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "admin_prefold",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    let mut games = scenario.start(&mut conn).await.games;
    games.sort_unstable_by_key(slot_id_for_game);
    let marker_game = games.remove(0);
    let adjudicated_game = games.remove(0);
    let marker_finished_at = marker_game
        .adjudicate_unstarted(
            &TournamentGameResult::Winner(Color::White),
            Conclusion::Committee,
            Utc::now(),
            &mut conn,
        )
        .await
        .expect("commit a terminal Game without resolving its Slot")
        .finished_at
        .expect("terminal fixture records finished_at");
    assert!(
        load_slot_for_game(scenario.tournament.id, marker_game.id, &mut conn)
            .await
            .expect("load durable recovery marker")
            .resolution
            .is_none()
    );

    let outcome = fixed_field_api::adjudicate_slot_atomic(
        scenario.tournament.id,
        slot_id_for_game(&adjudicated_game),
        TournamentGameResult::Winner(Color::Black),
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("adjudicate while folding the other committed terminal marker");
    assert!(outcome
        .commit
        .as_ref()
        .is_some_and(|commit| commit.finished_now));
    let commit = outcome
        .commit
        .as_ref()
        .expect("adjudication reports its commit");
    assert!(commit.standings_changed);
    assert!(!commit.format_changed);
    assert!(commit.catalog_changed);
    let mut expected_slot_ids = vec![
        slot_id_for_game(&marker_game),
        slot_id_for_game(&adjudicated_game),
    ];
    expected_slot_ids.sort_unstable();
    assert_eq!(commit.affected_slot_ids, expected_slot_ids);
    assert_eq!(
        load_slot_for_game(scenario.tournament.id, marker_game.id, &mut conn)
            .await
            .expect("reload folded marker Slot")
            .resolved_at,
        Some(marker_finished_at),
    );
    assert!(
        TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
            .await
            .expect("reload completed Round Robin Slots")
            .iter()
            .all(|slot| slot.resolution.is_some())
    );
    assert_eq!(
        Tournament::find(scenario.tournament.id, &mut conn)
            .await
            .expect("reload automatically finished tournament")
            .status(),
        TournamentStatus::Finished,
    );
    assert_eq!(
        final_outcome_count(scenario.tournament.id, &mut conn).await,
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn closeout_round_robin_is_authorized_atomic_and_finishes() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "closeout_rr",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;

    let not_started = fixed_field_api::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        &mut conn,
    )
    .await;
    assert!(matches!(not_started, Err(DbError::InvalidAction { .. })));

    let started = scenario.start(&mut conn).await;
    assert_eq!(started.games.len(), 1);
    let unauthorized = fixed_field_api::close_unstarted_slots(
        scenario.tournament.id,
        scenario.players[0],
        &mut conn,
    )
    .await;
    assert!(matches!(unauthorized, Err(DbError::Unauthorized)));

    let slots = TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("load closeout Slots")
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 2);
    let mut offer_ids = Vec::new();
    for slot in &slots {
        let (slot_offer_ids, _) = create_accepted_offer_with_pending_reschedule(
            scenario.tournament.id,
            slot.id,
            slot.white,
            slot.black,
            &mut conn,
        )
        .await;
        offer_ids.extend(slot_offer_ids);
    }
    diesel::update(users::table.find(scenario.players[0]))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .expect("grant fixture administrator access");

    let tournament_id = scenario.tournament.id;
    let admin_id = scenario.players[0];
    let rolled_back = conn
        .transaction::<_, DbError, _>(async move |tc| {
            let outcome =
                fixed_field_api::close_unstarted_slots(tournament_id, admin_id, tc).await?;
            assert_eq!(outcome.closed_slots, 2);
            assert_eq!(outcome.terminal_games.len(), 1);
            Err::<(), DbError>(DbError::InvalidAction {
                info: String::from("force closeout rollback"),
            })
        })
        .await;
    assert!(matches!(rolled_back, Err(DbError::InvalidAction { .. })));
    let rolled_back_game = Game::find_by_uuid(&started.games[0].id, &mut conn)
        .await
        .expect("reload rolled-back game");
    assert_eq!(rolled_back_game.finished, started.games[0].finished);
    assert_eq!(rolled_back_game.turn, started.games[0].turn);
    assert_eq!(rolled_back_game.game_status, started.games[0].game_status);
    assert_eq!(rolled_back_game.updated_at, started.games[0].updated_at);
    assert!(
        TournamentSlot::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .expect("reload rolled-back Slots")
            .into_iter()
            .all(|slot| slot.resolution.is_none())
    );
    assert_eq!(
        schedule_offers::table
            .filter(schedule_offers::id.eq_any(&offer_ids))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .expect("count rolled-back schedule offers"),
        4,
    );
    assert!(
        TournamentSlot::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .expect("reload rolled-back Slot appointments")
            .into_iter()
            .all(|slot| slot.scheduled_at.is_some())
    );

    let outcome =
        fixed_field_api::close_unstarted_slots(tournament_id, scenario.organizer_id, &mut conn)
            .await
            .expect("organizer closes Round Robin obligations");
    assert_eq!(outcome.closed_slots, 2);
    assert_eq!(outcome.terminal_games.len(), 1);
    let effects = outcome
        .commit
        .as_ref()
        .expect("closeout changed tournament state");
    assert_eq!(effects.schedule_updates.len(), 2);
    assert!(effects.finished_now);
    let terminal = Game::find_by_uuid(&started.games[0].id, &mut conn)
        .await
        .expect("reload closed game");
    assert!(terminal.finished);
    assert_eq!(terminal.conclusion, Conclusion::Forfeit.to_string());
    assert!(
        TournamentSlot::find_by_tournament_id(tournament_id, &mut conn)
            .await
            .expect("reload closed Slots")
            .into_iter()
            .all(|slot| { slot.scheduled_at.is_none() && slot.resolution.is_some() })
    );
    assert!(ScheduleOffer::find_for_tournament(tournament_id, &mut conn)
        .await
        .expect("load schedule offers after automatic tournament finish")
        .is_empty());
    let after_finish =
        fixed_field_api::close_unstarted_slots(tournament_id, scenario.organizer_id, &mut conn)
            .await;
    assert!(matches!(after_finish, Err(DbError::InvalidAction { .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn closeout_leaves_started_round_robin_games_untouched() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "closeout_started",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    let games = scenario.start(&mut conn).await.games;
    assert_eq!(games.len(), 2);
    execute(
        games[0].id,
        Command::StartReady {
            user_id: games[0].white_id,
        },
        &mut conn,
    )
    .await
    .expect("start one closeout fixture game");
    let started_before = Game::find_by_uuid(&games[0].id, &mut conn)
        .await
        .expect("load started fixture game");

    let outcome = fixed_field_api::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("close only unstarted game");
    assert_eq!(outcome.closed_slots, 1);
    assert_eq!(outcome.terminal_games[0].id, games[1].id);
    let started_after = Game::find_by_uuid(&games[0].id, &mut conn)
        .await
        .expect("reload started fixture game");
    assert_eq!(started_after.game_status, started_before.game_status);
    assert_eq!(started_after.updated_at, started_before.updated_at);
    assert_eq!(
        started_after.last_interaction,
        started_before.last_interaction
    );
    assert_eq!(started_after.conclusion, started_before.conclusion);
    assert!(!started_after.finished);
    assert!(
        load_slot_for_game(scenario.tournament.id, games[0].id, &mut conn)
            .await
            .expect("reload started game Slot")
            .resolution
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn command_after_deadline_commits_timeout_before_rejection_and_leaves_recovery_marker() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "deadline_atomic",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let game = scenario.start(&mut conn).await.games.remove(0);
    execute(
        game.id,
        Command::StartReady {
            user_id: game.white_id,
        },
        &mut conn,
    )
    .await
    .expect("start Ready game");

    let rejected = execute_at(
        game.id,
        Command::Move {
            user_id: game.white_id,
            turn: Turn::Move(
                "wA1".parse().expect("parse white ant"),
                Position::initial_spawn_position(),
            ),
            compensation: 0.0,
        },
        Utc::now() + Duration::days(1),
        &mut conn,
    )
    .await
    .expect("deadline settlement commits before rejection");
    let Outcome::TimedOut { game: terminal, .. } = rejected else {
        panic!("deadline settlement commits only the terminal Game")
    };
    let slot = load_slot_for_game(scenario.tournament.id, game.id, &mut conn)
        .await
        .expect("load timeout slot");
    assert!(slot.resolution.is_none());
    let finished_at = terminal.finished_at.expect("timeout records finished_at");
    let effects = reconcile_fixed_field_at(
        scenario.tournament.id,
        finished_at + Duration::minutes(1),
        &mut conn,
    )
    .await
    .expect("reconcile timeout marker")
    .expect("timeout reconciliation changes tournament state");
    assert!(effects.finished_now);
    let slot = load_slot_for_game(scenario.tournament.id, game.id, &mut conn)
        .await
        .expect("reload reconciled timeout slot");
    assert_eq!(slot.resolved_at, Some(finished_at));
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_deadline_candidate_is_rechecked_under_game_lock() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "dl_read",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let game = scenario.start(&mut conn).await.games.remove(0);
    execute(
        game.id,
        Command::StartReady {
            user_id: game.white_id,
        },
        &mut conn,
    )
    .await
    .expect("start Ready game");
    let game_id = game.id;
    let stale_observation = Utc::now();
    let refreshed_deadline = stale_observation + Duration::minutes(5);
    diesel::update(games::table.find(game_id))
        .set((
            games::last_interaction.eq(Some(stale_observation)),
            games::white_time_left.eq(Some(300_000_000_000_i64)),
            games::timeout_at.eq(Some(refreshed_deadline)),
        ))
        .execute(&mut conn)
        .await
        .expect("refresh deadline after the sweeper observed its candidate");
    let stale = execute_at(
        game_id,
        Command::SettleDeadline,
        stale_observation,
        &mut conn,
    )
    .await
    .expect("stale timeout candidate is rechecked under the authoritative Game lock");
    assert!(matches!(
        stale,
        Outcome::Applied {
            newly_terminal: false,
            ref game,
            ..
        } if !game.finished
    ));
    assert!(
        load_slot_for_game(scenario.tournament.id, game_id, &mut conn)
            .await
            .expect("reload slot after stale timeout")
            .resolution
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_start_materializes_round_one() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let prefix = "sw_start";
    let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
    let players = [
        create_user(&format!("{prefix}_p0"), &mut conn).await.id,
        create_user(&format!("{prefix}_p1"), &mut conn).await.id,
        create_user(&format!("{prefix}_p2"), &mut conn).await.id,
        create_user(&format!("{prefix}_p3"), &mut conn).await.id,
        create_user(&format!("{prefix}_p4"), &mut conn).await.id,
    ];
    for (extra, expected_rounds) in [(2, 4), (-2, 3)] {
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            &format!("{prefix}_{extra}"),
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Swiss(SwissConfig::automatic_swiss(extra, realtime_clock())),
            },
            &mut conn,
        )
        .await;
        let saved = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload saved Swiss configuration");
        let FormatConfig::Swiss(saved_config) = &saved.configuration().format else {
            panic!("saved Swiss tournament changed format");
        };
        assert_eq!(
            saved_config.rounds,
            SwissRoundConfiguration::automatic(extra)
        );
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let started = fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start Swiss tournament");

        assert_eq!(started.games.len(), 2);
        let FormatConfig::Swiss(configuration) = started.tournament.configuration().format.clone()
        else {
            panic!("started Swiss tournament changed format");
        };
        assert_eq!(
            configuration.rounds,
            SwissRoundConfiguration::Resolved {
                rounds: NonZeroU32::new(expected_rounds).unwrap(),
                requested_extra: Some(extra),
            },
        );
        let persisted = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload started Swiss configuration");
        assert_eq!(
            persisted.configuration(),
            started.tournament.configuration()
        );
        let rounds = TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load canonical Swiss round facts");
        assert_eq!(rounds.len(), 1);
        assert!(rounds[0].accepted_at >= tournament.created_at);
        assert_eq!(
            TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("load materialized Swiss slots")
                .len(),
            2
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_snapshot_uses_accepted_ratings_for_adjudicated_metrics() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("sw_metrics_o", &mut conn).await;
    let players = [
        create_user("sw_metrics_p0", &mut conn).await.id,
        create_user("sw_metrics_p1", &mut conn).await.id,
        create_user("sw_metrics_p2", &mut conn).await.id,
        create_user("sw_metrics_p3", &mut conn).await.id,
        create_user("sw_metrics_p4", &mut conn).await.id,
    ];
    let accepted_ratings = HashMap::from([
        (players[0], 1_400_u32),
        (players[1], 1_600_u32),
        (players[2], 1_800_u32),
        (players[3], 2_000_u32),
        (players[4], 2_200_u32),
    ]);
    for player in players {
        diesel::update(ratings::table.filter(ratings::user_uid.eq(player)))
            .set(ratings::rating.eq(f64::from(accepted_ratings[&player])))
            .execute(&mut conn)
            .await
            .expect("set a distinct accepted rating");
    }
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "sw_metrics",
        TournamentStatus::NotStarted,
        None,
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(SwissConfig::automatic_swiss(0, realtime_clock())),
        },
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Swiss metrics fixture");
    let slot = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load accepted Swiss slot")
        .into_iter()
        .next()
        .expect("Swiss round has a pairing");
    fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        slot.id,
        TournamentGameResult::Winner(Color::White),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("adjudicate accepted Swiss pairing");
    diesel::update(games::table.filter(games::tournament_slot_id.eq(Some(slot.id))))
        .set((
            games::white_rating.eq(Some(2_800.0)),
            games::black_rating.eq(Some(2_900.0)),
        ))
        .execute(&mut conn)
        .await
        .expect("make the later game snapshots differ from acceptance");

    let snapshot = load_by_id(tournament.id, &mut conn)
        .await
        .expect("project Swiss metrics fixture");
    let TournamentFormatResponse::Swiss { rounds, .. } = &snapshot.format else {
        panic!("Swiss metrics fixture has a Swiss projection");
    };
    let encounter = rounds[0]
        .encounters
        .iter()
        .find(|encounter| {
            encounter
                .slots
                .iter()
                .any(|candidate| candidate.id == slot.id)
        })
        .expect("project the adjudicated pairing");
    assert_eq!(
        encounter.rating_snapshots,
        [accepted_ratings[&slot.white], accepted_ratings[&slot.black]].map(Some),
    );
    let bye_ = &rounds[0].byes[0];
    assert_eq!(bye_.rating_snapshot, Some(accepted_ratings[&bye_.player]));

    let white = snapshot
        .player_stats
        .iter()
        .find(|metrics| metrics.player == slot.white)
        .expect("project white metrics");
    assert_eq!(
        white.average_opponent_rating,
        Some(accepted_ratings[&slot.black]),
    );
    assert_eq!(
        white.performance_rating,
        i32::try_from(accepted_ratings[&slot.black])
            .ok()
            .map(|rating| rating + 500),
    );
    assert_eq!(
        (white.games_played, white.wins, white.draws, white.losses),
        (0, 1, 0, 0)
    );
    let black = snapshot
        .player_stats
        .iter()
        .find(|metrics| metrics.player == slot.black)
        .expect("project black metrics");
    assert_eq!(
        black.average_opponent_rating,
        Some(accepted_ratings[&slot.white]),
    );
    assert_eq!(
        black.performance_rating,
        i32::try_from(accepted_ratings[&slot.white])
            .ok()
            .map(|rating| rating - 500),
    );
    assert_eq!(
        (black.games_played, black.wins, black.draws, black.losses),
        (0, 0, 0, 1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_progression_finishes_when_projection_reports_exhaustion() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    {
        let prefix = "sw_exhaustion";
        let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
        let players = [
            create_user(&format!("{prefix}_p0"), &mut conn).await.id,
            create_user(&format!("{prefix}_p1"), &mut conn).await.id,
            create_user(&format!("{prefix}_p2"), &mut conn).await.id,
            create_user(&format!("{prefix}_p3"), &mut conn).await.id,
            create_user(&format!("{prefix}_p4"), &mut conn).await.id,
        ];
        let tournament = Tournament::create(
            organizer.id,
            &NewTournament::new(TournamentDetails {
                name: prefix.to_string(),
                description: None,
                seats: Some(5),
                min_seats: 5,
                invite_only: false,
                band_upper: None,
                band_lower: None,
                starts_at: None,
                configuration: Config {
                    bot_admission: BotAdmission::HumansAndBots,
                    format: FormatConfig::Swiss(SwissConfig::automatic_swiss(0, realtime_clock())),
                },
            })
            .expect("build exhausted Swiss pairing fixture"),
            &mut conn,
        )
        .await
        .expect("create exhausted Swiss pairing fixture");
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start Swiss tournament");
        let slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load first Swiss round")
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(slots.len(), 2);

        for player_id in players
            .iter()
            .copied()
            .filter(|player| *player != slots[0].white && *player != slots[0].black)
        {
            fixed_field_db::withdraw_player(tournament.id, player_id, organizer.id, &mut conn)
                .await
                .expect("withdraw one side of the other first-round pairing");
        }
        let game_id = game_id_for_slot(tournament.id, slots[0].id, &mut conn)
            .await
            .expect("first-round Slot has a Hive Game");
        execute(
            game_id,
            Command::StartReady {
                user_id: slots[0].white,
            },
            &mut conn,
        )
        .await
        .expect("start the final first-round game");
        for (actor, piece, position) in [
            (slots[0].white, "wA1", Position::initial_spawn_position()),
            (
                slots[0].black,
                "bA1",
                Position::initial_spawn_position().to(Direction::E),
            ),
        ] {
            execute(
                game_id,
                Command::Move {
                    user_id: actor,
                    turn: Turn::Move(piece.parse().expect("parse ant"), position),
                    compensation: 0.0,
                },
                &mut conn,
            )
            .await
            .expect("play the final first-round game");
        }
        let terminal = execute(
            game_id,
            Command::Control {
                user_id: slots[0].black,
                control: GameControl::Resign(Color::Black),
            },
            &mut conn,
        )
        .await
        .expect("resolve the final first-round game through play");
        let Outcome::Applied {
            game: terminal,
            newly_terminal: true,
            ..
        } = terminal
        else {
            panic!("terminal tournament Game leaves a reconciliation marker")
        };
        let effects = reconcile_fixed_field_at(
            tournament.id,
            terminal.finished_at.expect("terminal Game has finished_at") + Duration::minutes(1),
            &mut conn,
        )
        .await
        .expect("reconcile exhausted Swiss field")
        .expect("reconciliation finishes the tournament");

        assert!(effects.finished_now);
        assert_eq!(
            Tournament::find(tournament.id, &mut conn)
                .await
                .expect("reload early-finished Swiss tournament")
                .status(),
            TournamentStatus::Finished,
        );
        assert_eq!(
            TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("reload accepted Swiss rounds")
                .len(),
            1,
        );
        assert!(TournamentFinalOutcome::load(tournament.id, &mut conn)
            .await
            .expect("load early final standings")
            .is_some());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_progression_automatically_accepts_a_successful_post_forfeit_projection() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("sw_forfeit_o", &mut conn).await;
    let players = [
        create_user("sw_forfeit_p0", &mut conn).await.id,
        create_user("sw_forfeit_p1", &mut conn).await.id,
        create_user("sw_forfeit_p2", &mut conn).await.id,
        create_user("sw_forfeit_p3", &mut conn).await.id,
        create_user("sw_forfeit_p4", &mut conn).await.id,
    ];
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "sw_forfeit",
        TournamentStatus::NotStarted,
        None,
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(SwissConfig::automatic_swiss(0, realtime_clock())),
        },
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Swiss tournament");

    let first_round_slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load first Swiss round")
        .into_iter()
        .collect::<Vec<_>>();
    for slot in first_round_slots {
        let outcome = fixed_field_api::adjudicate_slot_atomic(
            tournament.id,
            slot.id,
            TournamentGameResult::DoubleForfeit,
            organizer.id,
            &mut conn,
        )
        .await
        .expect("record a complete-match forfeit");
        let commit = outcome
            .commit
            .expect("Swiss adjudication reports its changes");
        assert!(commit.affected_slot_ids.contains(&slot.id));
        assert!(commit.standings_changed);
        assert!(commit.format_changed);
        assert!(commit.catalog_changed);
    }

    assert_eq!(
        TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load automatically accepted Swiss rounds")
            .len(),
        2,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_round_persists_bye_and_applies_release_policy_to_both_legs() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    for (prefix, release_policy, expected_games) in [
        ("ds_unlocked", ReleasePolicy::FullyUnlocked, 4),
        ("ds_sequential", ReleasePolicy::SequentialPerMatchup, 2),
    ] {
        let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
        let players = [
            create_user(&format!("{prefix}_p0"), &mut conn).await.id,
            create_user(&format!("{prefix}_p1"), &mut conn).await.id,
            create_user(&format!("{prefix}_p2"), &mut conn).await.id,
            create_user(&format!("{prefix}_p3"), &mut conn).await.id,
            create_user(&format!("{prefix}_p4"), &mut conn).await.id,
        ];
        let mut swiss = SwissConfig::automatic_double_swiss(
            0,
            realtime_clock(),
            DoubleSwissPrimaryScore::GamePoints,
        );
        swiss.double_swiss_release_policy = release_policy;
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            prefix,
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Swiss(swiss),
            },
            &mut conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let started = fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start Double-Swiss tournament");
        assert_eq!(started.games.len(), expected_games);

        let rounds = TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load accepted round");
        assert_eq!(rounds.len(), 1);
        let pairings = rounds[0].pairings();
        assert_eq!(pairings.games.len(), 2);
        assert_eq!(pairings.byes.len(), 1);
        assert_eq!(
            TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
                .await
                .expect("load reciprocal slots")
                .len(),
            4,
        );
        if release_policy == ReleasePolicy::SequentialPerMatchup {
            let snapshot = load_by_id(tournament.id, &mut conn)
                .await
                .expect("project sequential Double-Swiss round");
            let TournamentFormatResponse::Swiss { rounds, .. } = snapshot.format else {
                panic!("Double-Swiss snapshot has a Swiss projection");
            };
            let first_leg_slot_ids = rounds[0]
                .encounters
                .iter()
                .map(|encounter| {
                    assert_eq!(encounter.slots.len(), 2);
                    let first = &encounter.slots[0];
                    let second = &encounter.slots[1];
                    assert!(first.game.is_some());
                    assert!(second.game.is_none());
                    assert_eq!(second.waits_for, Some(first.id));
                    first.id
                })
                .collect::<Vec<_>>();
            for slot_id in first_leg_slot_ids {
                fixed_field_api::adjudicate_slot_atomic(
                    tournament.id,
                    slot_id,
                    TournamentGameResult::Winner(Color::White),
                    organizer.id,
                    &mut conn,
                )
                .await
                .expect("resolve a Double-Swiss first leg");
            }
            let released = load_by_id(tournament.id, &mut conn)
                .await
                .expect("reproject released Double-Swiss second legs");
            let TournamentFormatResponse::Swiss { rounds, .. } = released.format else {
                panic!("Double-Swiss snapshot has a Swiss projection");
            };
            assert!(rounds[0].encounters.iter().all(|encounter| {
                encounter.slots[1].game.is_some() && encounter.slots[1].waits_for.is_none()
            }));
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn double_swiss_return_leg_keeps_its_identity_through_correction_and_completion() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("ds_return_o", &mut conn).await;
    let mut players = Vec::new();
    for index in 0..5 {
        players.push(
            create_user(&format!("ds_return_p{index}"), &mut conn)
                .await
                .id,
        );
    }
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "double_swiss_return_first",
        TournamentStatus::NotStarted,
        None,
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                0,
                realtime_clock(),
                DoubleSwissPrimaryScore::GamePoints,
            )),
        },
        &mut conn,
    )
    .await;
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Double Swiss");
    let initial = load_by_id(tournament.id, &mut conn)
        .await
        .expect("load initial snapshot");
    let TournamentFormatResponse::Swiss { rounds, .. } = initial.format else {
        panic!("Double Swiss has a Swiss projection");
    };
    let encounter = &rounds[0].encounters[0];
    let [first, second] = encounter.slots.as_slice() else {
        panic!("Double Swiss has two ordered legs");
    };
    let first_id = first.id;
    let second_id = second.id;
    let participants = encounter.participants;
    assert_eq!(first.participants, participants);
    assert_eq!(second.participants, [participants[1], participants[0]]);

    for (result, expected_points, expected_outcome) in [
        (
            TournamentGameResult::Winner(Color::White),
            [0, 2],
            Some(GameOutcome::Adjudicated(white_forfeit_win())),
        ),
        (
            TournamentGameResult::Winner(Color::Black),
            [2, 0],
            Some(GameOutcome::Adjudicated(black_forfeit_win())),
        ),
        (TournamentGameResult::Unknown, [0, 0], None),
        (
            TournamentGameResult::Winner(Color::White),
            [0, 2],
            Some(GameOutcome::Adjudicated(white_forfeit_win())),
        ),
    ] {
        fixed_field_api::adjudicate_slot_atomic(
            tournament.id,
            second_id,
            result,
            organizer.id,
            &mut conn,
        )
        .await
        .expect("update return leg while the first leg is unresolved");
        let snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("project return leg");
        let TournamentFormatResponse::Swiss { rounds, .. } = snapshot.format else {
            panic!("Double Swiss has a Swiss projection");
        };
        assert_eq!(rounds.len(), 1);
        let encounter = rounds[0]
            .encounters
            .iter()
            .find(|encounter| encounter.participants == participants)
            .expect("find the accepted encounter");
        let first = encounter
            .slots
            .iter()
            .find(|slot| slot.id == first_id)
            .unwrap();
        let second = encounter
            .slots
            .iter()
            .find(|slot| slot.id == second_id)
            .unwrap();
        assert_eq!(first.outcome, None);
        assert_eq!(first.awarded_game_points, None);
        assert_eq!(second.outcome, expected_outcome);
        assert_eq!(encounter.completion, None);
        let rows = snapshot
            .standings
            .expect("live standings")
            .groups
            .into_iter()
            .flat_map(|group| group.rows)
            .map(|row| (row.user_id, row))
            .collect::<HashMap<_, _>>();
        for (participant, points) in participants.into_iter().zip(expected_points) {
            assert_eq!(
                rows[&participant].primary_score,
                StandingValue::Score(Score::new(points))
            );
        }
    }

    fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        first_id,
        TournamentGameResult::Winner(Color::White),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("complete the first leg after the return leg");
    let completed = load_by_id(tournament.id, &mut conn)
        .await
        .expect("project completed encounter");
    let TournamentFormatResponse::Swiss { rounds, .. } = completed.format else {
        panic!("Double Swiss has a Swiss projection");
    };
    let encounter = rounds[0]
        .encounters
        .iter()
        .find(|encounter| encounter.participants == participants)
        .expect("find the completed encounter");
    let completion = encounter.completion.expect("both legs complete the match");
    assert_eq!(completion.game_points, [Score::new(2), Score::new(2)]);
    let rows = completed
        .standings
        .expect("live standings")
        .groups
        .into_iter()
        .flat_map(|group| group.rows)
        .map(|row| (row.user_id, row))
        .collect::<HashMap<_, _>>();
    for participant in participants {
        assert_eq!(
            rows[&participant].primary_score,
            StandingValue::Score(Score::new(2))
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_projection_uses_the_configured_pre_round_primary_score() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");

    for (prefix, primary_score) in [
        ("sw_prefix_game", DoubleSwissPrimaryScore::GamePoints),
        ("sw_prefix_match", DoubleSwissPrimaryScore::MatchPoints),
    ] {
        let organizer = create_user(&format!("{prefix}_o"), &mut conn).await;
        let players = [
            create_user(&format!("{prefix}_p0"), &mut conn).await.id,
            create_user(&format!("{prefix}_p1"), &mut conn).await.id,
            create_user(&format!("{prefix}_p2"), &mut conn).await.id,
            create_user(&format!("{prefix}_p3"), &mut conn).await.id,
            create_user(&format!("{prefix}_p4"), &mut conn).await.id,
        ];
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            prefix,
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                    0,
                    realtime_clock(),
                    primary_score,
                )),
            },
            &mut conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let started = fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start Double-Swiss prefix-score fixture");
        assert_eq!(started.games.len(), 4);

        let first_round_slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load first-round Double-Swiss slots")
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(first_round_slots.len(), 4);
        for slot in first_round_slots {
            fixed_field_api::adjudicate_slot_atomic(
                tournament.id,
                slot.id,
                TournamentGameResult::Winner(Color::White),
                organizer.id,
                &mut conn,
            )
            .await
            .expect("complete a first-round Double-Swiss leg");
        }
        let snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("project accepted Double-Swiss history");
        let TournamentFormatResponse::Swiss { rounds, .. } = snapshot.format else {
            panic!("Double-Swiss snapshot has a Swiss projection");
        };
        assert_eq!(rounds.len(), 2);
        let first_round = &rounds[0];
        let mut expected_prefix = HashMap::<Uuid, Score>::new();
        for encounter in &first_round.encounters {
            assert_eq!(encounter.pre_round_primary_scores, [Score::new(0); 2]);
            let first_game = encounter.slots[0]
                .game
                .as_ref()
                .expect("accepted encounter retains its first game");
            assert_eq!(encounter.participants, first_game.participants);
            assert_eq!(encounter.rating_snapshots, [Some(1_500); 2]);
            let completion = encounter
                .completion
                .expect("completed encounter projects aggregate awards");
            assert_ne!(completion.game_points, completion.match_points);
            let awards = if primary_score == DoubleSwissPrimaryScore::MatchPoints {
                completion.match_points
            } else {
                completion.game_points
            };
            for (index, participant) in encounter.participants.into_iter().enumerate() {
                assert!(expected_prefix.insert(participant, awards[index]).is_none());
            }
        }
        for bye_ in &first_round.byes {
            assert_eq!(bye_.pre_round_primary_score, Score::new(0));
            assert_eq!(bye_.rating_snapshot, Some(1_500));
            assert_ne!(bye_.game_points, bye_.match_points);
            let award = if primary_score == DoubleSwissPrimaryScore::MatchPoints {
                bye_.match_points
            } else {
                bye_.game_points
            };
            assert!(expected_prefix.insert(bye_.player, award).is_none());
        }
        assert_eq!(expected_prefix.len(), players.len());

        let second_round = &rounds[1];
        for encounter in &second_round.encounters {
            assert_eq!(
                encounter.pre_round_primary_scores,
                encounter
                    .participants
                    .map(|participant| expected_prefix[&participant])
            );
        }
        for bye_ in &second_round.byes {
            assert_eq!(bye_.pre_round_primary_score, expected_prefix[&bye_.player]);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn swiss_result_correction_updates_slot_and_game_consistently() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("sw_front_o", &mut conn).await;
    let players = [
        create_user("sw_front_p0", &mut conn).await.id,
        create_user("sw_front_p1", &mut conn).await.id,
        create_user("sw_front_p2", &mut conn).await.id,
        create_user("sw_front_p3", &mut conn).await.id,
        create_user("sw_front_p4", &mut conn).await.id,
    ];
    let tournament = Tournament::create(
        organizer.id,
        &NewTournament::new(TournamentDetails {
            name: String::from("swiss_correction_frontier"),
            description: Some(String::from(
                "A deterministic Swiss correction frontier fixture used for database behavior tests.",
            )),
            seats: Some(5),
            min_seats: 5,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Swiss(SwissConfig::automatic_swiss(
                    0,
                    realtime_clock(),
                )),
            },
        })
        .expect("build Swiss correction tournament"),
        &mut conn,
    )
    .await
    .expect("create Swiss correction tournament");
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
    fixed_field_api::start_by_organizer(tournament.id, organizer.id, &mut conn)
        .await
        .expect("start Swiss correction scenario");

    let first_round_slots = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
        .await
        .expect("load first-round slots")
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(first_round_slots.len(), 2);
    fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        first_round_slots[0].id,
        TournamentGameResult::Winner(Color::White),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("adjudicate one first-round result");
    assert_eq!(
        TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load incomplete Swiss round")
            .len(),
        1,
    );

    let corrected_slot_id = first_round_slots[0].id;
    let replacement_outcome = GameOutcome::Adjudicated(black_forfeit_win());
    let correction = fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        corrected_slot_id,
        TournamentGameResult::Winner(Color::Black),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("correct a result while its round is the frontier");
    let correction_commit = correction.commit.expect("correction reports exact changes");
    assert_eq!(correction_commit.affected_slot_ids, vec![corrected_slot_id]);
    assert!(correction_commit.standings_changed);
    assert!(correction_commit.format_changed);
    assert!(!correction_commit.catalog_changed);
    let corrected_slot = TournamentSlot::find(tournament.id, corrected_slot_id, &mut conn)
        .await
        .expect("reload corrected frontier slot");
    assert!(corrected_slot.resolution == Some(Resolution::Result(replacement_outcome)));
    let corrected_game_id = game_id_for_slot(tournament.id, corrected_slot_id, &mut conn)
        .await
        .expect("corrected Slot keeps its game");
    assert_eq!(
        Game::find_by_uuid(&corrected_game_id, &mut conn)
            .await
            .expect("reload corrected game")
            .tournament_game_result,
        TournamentGameResult::Winner(Color::Black).to_string(),
    );

    fixed_field_api::adjudicate_slot_atomic(
        tournament.id,
        first_round_slots[1].id,
        TournamentGameResult::Winner(Color::White),
        organizer.id,
        &mut conn,
    )
    .await
    .expect("complete the first round and advance automatically");
    assert_eq!(
        TournamentSwissRound::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("reload automatically advanced Swiss rounds")
            .len(),
        2,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn finish_snapshot_and_lifecycle_share_one_transaction() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "finish_atomic",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let started = scenario.start(&mut conn).await;
    let memberships = TournamentUser::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("load tournament memberships");
    let users = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect::<Vec<_>>();
    let users = [users[0], users[1]];
    let snapshot = round_robin_snapshot(scenario.tournament.id, users);
    let started_at = started.tournament.started_at.expect("started at");
    let tournament_id = scenario.tournament.id;

    let invalid = conn
        .transaction::<_, DbError, _>(async move |tc| {
            let state = load_in_progress_for_update(tournament_id, &[], BotUsers::Skip, tc).await?;
            persist_finished_with_outcome(
                &state.tournament,
                snapshot,
                started_at - Duration::seconds(1),
                tc,
            )
            .await
            .map(|_| ())
        })
        .await;
    assert!(invalid.is_err());
    assert_eq!(final_outcome_count(tournament_id, &mut conn).await, 0);

    let expected = round_robin_snapshot(tournament_id, users);
    let persisted = expected.clone();
    let finished = conn
        .transaction::<_, DbError, _>(async move |tc| {
            let state = load_in_progress_for_update(tournament_id, &[], BotUsers::Skip, tc).await?;
            persist_finished_with_outcome(
                &state.tournament,
                persisted,
                started_at + Duration::seconds(1),
                tc,
            )
            .await
        })
        .await
        .expect("finish tournament");
    assert_eq!(finished.status(), TournamentStatus::Finished);
    let stored = TournamentFinalOutcome::load(tournament_id, &mut conn)
        .await
        .expect("load persisted final outcome")
        .expect("final outcome exists after commit");
    assert_eq!(stored.standings, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn organizer_start_rejects_a_stale_or_duplicate_reviewed_roster_atomically() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "intent_start",
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;

    let stale = fixed_field_db::start_by_organizer(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![scenario.players[0]],
        &mut conn,
    )
    .await;
    assert!(matches!(stale, Err(DbError::InvalidAction { .. })));
    assert_start_rollback_restored_state(scenario.tournament.id, &mut conn).await;

    let duplicate = fixed_field_db::start_by_organizer(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![scenario.players[0], scenario.players[0]],
        &mut conn,
    )
    .await;
    assert!(matches!(duplicate, Err(DbError::InvalidAction { .. })));
    assert_start_rollback_restored_state(scenario.tournament.id, &mut conn).await;

    assert!(scenario.start(&mut conn).await.started_now);
}

#[tokio::test(flavor = "multi_thread")]
async fn closeout_closes_only_reviewed_slots_and_rolls_back_an_invalid_batch() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "intent_close",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    scenario.start(&mut conn).await;
    let slots = TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("load closeout slots");
    assert_eq!(slots.len(), 2);

    let invalid = fixed_field_db::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![slots[0].id, Uuid::new_v4()],
        &mut conn,
    )
    .await;
    assert!(matches!(invalid, Err(DbError::InvalidAction { .. })));
    assert!(
        TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
            .await
            .expect("reload rolled-back closeout slots")
            .iter()
            .all(|slot| slot.resolution.is_none())
    );

    let duplicate = fixed_field_db::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![slots[0].id, slots[0].id],
        &mut conn,
    )
    .await;
    assert!(matches!(duplicate, Err(DbError::InvalidAction { .. })));
    assert!(
        TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
            .await
            .expect("reload duplicate-rejected closeout slots")
            .iter()
            .all(|slot| slot.resolution.is_none())
    );

    let outcome = fixed_field_db::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![slots[0].id],
        &mut conn,
    )
    .await
    .expect("close reviewed slot only");
    assert_eq!(outcome.closed_slots, 1);
    assert!(
        TournamentSlot::find(scenario.tournament.id, slots[0].id, &mut conn,)
            .await
            .expect("reload reviewed slot")
            .resolution
            .is_some()
    );
    assert!(
        TournamentSlot::find(scenario.tournament.id, slots[1].id, &mut conn,)
            .await
            .expect("reload unreviewed slot")
            .resolution
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sequential_closeout_reports_the_newly_unblocked_successor() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "intent_seq_close",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
        &mut conn,
    )
    .await;
    let started = scenario.start(&mut conn).await;
    assert_eq!(started.games.len(), 1);
    let predecessor_id = slot_id_for_game(&started.games[0]);
    let successor_id = TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("load sequential closeout slots")
        .into_iter()
        .find(|slot| slot.id != predecessor_id)
        .expect("find the planned successor")
        .id;

    let outcome = fixed_field_db::close_unstarted_slots(
        scenario.tournament.id,
        scenario.organizer_id,
        vec![predecessor_id],
        &mut conn,
    )
    .await
    .expect("close only the reviewed predecessor");
    let commit = outcome
        .commit
        .expect("closeout reports its projection effects");
    assert!(commit.availability_changed);
    let mut expected_slot_ids = vec![predecessor_id, successor_id];
    expected_slot_ids.sort_unstable();
    assert_eq!(commit.affected_slot_ids, expected_slot_ids);
}

#[tokio::test(flavor = "multi_thread")]
async fn adjudication_requires_the_exact_reviewed_resolution() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let scenario = RoundRobinScenario::unstarted(
        "intent_adj",
        NonZeroU32::new(2).unwrap(),
        ReleasePolicy::FullyUnlocked,
        &mut conn,
    )
    .await;
    scenario.start(&mut conn).await;
    let slot = TournamentSlot::find_by_tournament_id(scenario.tournament.id, &mut conn)
        .await
        .expect("load adjudication slot")
        .remove(0);
    let first = Resolution::Result(GameOutcome::Adjudicated(white_forfeit_win()));
    let second = Resolution::Result(GameOutcome::Adjudicated(black_forfeit_win()));

    let initial = fixed_field_db::adjudicate_slot_atomic(
        scenario.tournament.id,
        slot.id,
        TournamentGameResult::Winner(Color::White),
        None,
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("initial exact adjudication");
    assert!(initial
        .commit
        .as_ref()
        .is_some_and(|commit| commit.availability_changed));

    let duplicate = fixed_field_db::adjudicate_slot_atomic(
        scenario.tournament.id,
        slot.id,
        TournamentGameResult::Winner(Color::White),
        None,
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("duplicate desired result is a no-op");
    assert!(!duplicate.cleared && !duplicate.newly_terminal && duplicate.commit.is_none());

    let stale_none = fixed_field_db::adjudicate_slot_atomic(
        scenario.tournament.id,
        slot.id,
        TournamentGameResult::Winner(Color::Black),
        None,
        scenario.organizer_id,
        &mut conn,
    )
    .await;
    assert!(matches!(stale_none, Err(DbError::InvalidAction { .. })));

    let correction = fixed_field_db::adjudicate_slot_atomic(
        scenario.tournament.id,
        slot.id,
        TournamentGameResult::Winner(Color::Black),
        Some(first),
        scenario.organizer_id,
        &mut conn,
    )
    .await
    .expect("exact prior resolution permits correction");
    assert!(correction
        .commit
        .as_ref()
        .is_some_and(|commit| !commit.availability_changed));

    for result in [
        TournamentGameResult::Unknown,
        TournamentGameResult::Winner(Color::White),
    ] {
        let stale = fixed_field_db::adjudicate_slot_atomic(
            scenario.tournament.id,
            slot.id,
            result,
            Some(first),
            scenario.organizer_id,
            &mut conn,
        )
        .await;
        assert!(matches!(stale, Err(DbError::InvalidAction { .. })));
    }
    assert_eq!(
        TournamentSlot::find(scenario.tournament.id, slot.id, &mut conn)
            .await
            .expect("reload protected adjudication")
            .resolution,
        Some(second),
    );
}

async fn assert_start_rollback_restored_state(tournament_id: Uuid, conn: &mut DbConn<'_>) {
    let tournament = Tournament::find(tournament_id, conn)
        .await
        .expect("reload tournament");
    assert_eq!(tournament.status(), TournamentStatus::NotStarted);
    assert!(tournament.started_at.is_none());
    assert!(TournamentSlot::find_by_tournament_id(tournament_id, conn)
        .await
        .expect("reload slots")
        .is_empty());
    assert!(tournament
        .games(conn)
        .await
        .expect("reload games")
        .is_empty());
    assert!(TournamentUser::find_by_tournament_id(tournament_id, conn)
        .await
        .expect("reload unstarted memberships")
        .iter()
        .all(|membership| {
            membership.pairing_number.is_none()
                && membership.arena_rating.is_none()
                && membership.arena_pairing_intent.is_none()
                && membership.arena_waiting_since.is_none()
        }));
}

fn slot_id_for_game(game: &Game) -> Uuid {
    game.tournament_slot_id
        .expect("fixed-field fixture game has a Slot ID")
}

async fn create_accepted_offer_with_pending_reschedule(
    tournament_id: Uuid,
    slot_id: Uuid,
    proposer_id: Uuid,
    opponent_id: Uuid,
    conn: &mut DbConn<'_>,
) -> ([Uuid; 2], DateTime<Utc>) {
    let now = Utc::now();
    let proposed_time = now + Duration::hours(1);
    let accepted_offer = ScheduleOffer::propose(
        proposer_id,
        tournament_id,
        slot_id,
        vec![proposed_time],
        conn,
    )
    .await
    .expect("propose accepted schedule offer")
    .pop()
    .expect("accepted schedule offer insertion is returned");
    let selected_time = accepted_offer
        .candidates()
        .expect("load persisted candidate times")
        .into_iter()
        .next()
        .expect("accepted offer has one candidate");
    ScheduleOffer::accept(accepted_offer.id, opponent_id, selected_time, conn)
        .await
        .expect("accept schedule offer");
    let pending_offer = ScheduleOffer::propose(
        opponent_id,
        tournament_id,
        slot_id,
        vec![now + Duration::hours(2), now + Duration::hours(3)],
        conn,
    )
    .await
    .expect("propose reschedule offer")
    .pop()
    .expect("pending reschedule offer insertion is returned");
    ([accepted_offer.id, pending_offer.id], selected_time)
}

fn black_forfeit_win() -> AdjudicatedGameOutcome {
    AdjudicatedGameOutcome::new(
        AdjudicatedSideResult::ForfeitLoss,
        AdjudicatedSideResult::ForfeitWin,
    )
    .unwrap()
}

fn white_forfeit_win() -> AdjudicatedGameOutcome {
    AdjudicatedGameOutcome::new(
        AdjudicatedSideResult::ForfeitWin,
        AdjudicatedSideResult::ForfeitLoss,
    )
    .unwrap()
}

async fn load_slot_for_game(
    tournament_id: Uuid,
    game_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<TournamentSlot, DbError> {
    let game = Game::find_by_uuid(&game_id, conn).await?;
    TournamentSlot::find(tournament_id, slot_id_for_game(&game), conn).await
}

async fn game_id_for_slot(
    tournament_id: Uuid,
    slot_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Option<Uuid> {
    games::table
        .filter(games::tournament_id.eq(tournament_id))
        .filter(games::tournament_slot_id.eq(slot_id))
        .select(games::id)
        .first(conn)
        .await
        .optional()
        .expect("query fixture Game by Slot ID")
}

async fn invitation_count(tournament_id: Uuid, conn: &mut DbConn<'_>) -> i64 {
    tournaments_invitations::table
        .filter(tournaments_invitations::tournament_id.eq(tournament_id))
        .count()
        .get_result(conn)
        .await
        .expect("count outstanding invitations")
}

async fn final_outcome_count(tournament_id: Uuid, conn: &mut DbConn<'_>) -> i64 {
    tournament_final_outcomes::table
        .filter(tournament_final_outcomes::tournament_id.eq(tournament_id))
        .count()
        .get_result(conn)
        .await
        .expect("count final outcomes")
}
