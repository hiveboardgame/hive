use super::{
    double_swiss::{map_double_swiss, DoubleSwissInput},
    model::{
        AuditDiagnostic,
        AuditOutcome,
        AuditReport,
        AuditSeverity,
        LegacyConfigurationReport,
        LegacySource,
        SupportedLegacyMapping,
        TournamentAuditReport,
        TournamentPlan,
    },
    round_robin::{map_round_robin, RoundRobinInput},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::Hash,
};
use uuid::Uuid;

pub fn run(source: &LegacySource) -> AuditOutcome {
    let mut diagnostics = validate_relations(source);
    let mut plans = Vec::new();
    let mut reports = Vec::new();
    let mut tournaments = source.tournaments.iter().collect::<Vec<_>>();
    tournaments.sort_by_key(|tournament| tournament.id);

    for tournament in tournaments {
        let memberships = source
            .memberships
            .iter()
            .filter(|row| row.tournament_id == tournament.id)
            .copied()
            .collect::<Vec<_>>();
        let games = source
            .games
            .iter()
            .filter(|row| row.tournament_id == Some(tournament.id))
            .cloned()
            .collect::<Vec<_>>();
        let intended_mapping = intended_mapping(&tournament.mode);
        let mapped = match intended_mapping {
            Some(SupportedLegacyMapping::RoundRobin) => map_round_robin(&RoundRobinInput {
                tournament,
                memberships: &memberships,
                games: &games,
            })
            .map(|mapped| (mapped.plan, mapped.diagnostics)),
            Some(SupportedLegacyMapping::FinishedDoubleSwiss) => {
                map_double_swiss(&DoubleSwissInput {
                    tournament,
                    memberships: &memberships,
                    games: &games,
                })
                .map(|plan| (plan, Vec::new()))
            }
            None => Err(vec![AuditDiagnostic::hard_failure(
                Some(tournament.id),
                "unsupported_legacy_mode",
                format!(
                    "legacy tournament mode {:?} has no deliberate mapping",
                    tournament.mode
                ),
            )]),
        };
        let mapped_plan = match mapped {
            Ok((plan, mut mapping_diagnostics)) => {
                diagnostics.append(&mut mapping_diagnostics);
                Some(plan)
            }
            Err(mut failures) => {
                diagnostics.append(&mut failures);
                None
            }
        };
        reports.push(tournament_report(
            source,
            tournament.id,
            intended_mapping,
            mapped_plan.as_ref(),
        ));
        if let Some(plan) = mapped_plan {
            plans.push(plan);
        }
    }

    plans.sort_by_key(|plan| plan.tournament.id);
    reports.sort_by_key(|report| report.tournament_id);
    diagnostics.sort();
    diagnostics.dedup();
    let counts = audit_counts(source, &reports, &plans, &diagnostics);
    AuditOutcome {
        report: AuditReport {
            source: source.metadata.clone(),
            tournaments: reports,
            counts,
            diagnostics,
        },
        plans,
    }
}

fn intended_mapping(mode: &str) -> Option<SupportedLegacyMapping> {
    match mode {
        "DoubleRoundRobin" | "QuadrupleRoundRobin" | "SextupleRoundRobin" => {
            Some(SupportedLegacyMapping::RoundRobin)
        }
        "DoubleSwiss" => Some(SupportedLegacyMapping::FinishedDoubleSwiss),
        _ => None,
    }
}

fn validate_relations(source: &LegacySource) -> Vec<AuditDiagnostic> {
    let mut diagnostics = Vec::new();
    let tournament_ids = unique_uuid_ids(
        source.tournaments.iter().map(|row| row.id),
        "duplicate_tournament",
        "tournament",
        &mut diagnostics,
    );
    unique_uuid_ids(
        source.games.iter().map(|row| row.id),
        "duplicate_game",
        "game",
        &mut diagnostics,
    );
    let series_ids = unique_uuid_ids(
        source.series.iter().map(|row| row.id),
        "duplicate_series",
        "series",
        &mut diagnostics,
    );

    validate_tournament_relations(source, &tournament_ids, &series_ids, &mut diagnostics);
    validate_game_relations(source, &tournament_ids, &mut diagnostics);
    validate_schedules(source, &tournament_ids, &mut diagnostics);
    validate_series(source, &series_ids, &mut diagnostics);
    diagnostics
}

fn validate_tournament_relations(
    source: &LegacySource,
    tournament_ids: &HashSet<Uuid>,
    series_ids: &HashSet<Uuid>,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    let mut keys = HashSet::new();
    for row in &source.memberships {
        duplicate_association(
            &mut keys,
            (row.tournament_id, row.user_id),
            Some(row.tournament_id),
            "duplicate_membership",
            "membership",
            diagnostics,
        );
        require_tournament(
            tournament_ids.contains(&row.tournament_id),
            row.tournament_id,
            "membership",
            diagnostics,
        );
    }

    keys.clear();
    for row in &source.organizers {
        duplicate_association(
            &mut keys,
            (row.tournament_id, row.organizer_id),
            Some(row.tournament_id),
            "duplicate_organizer",
            "organizer",
            diagnostics,
        );
        require_tournament(
            tournament_ids.contains(&row.tournament_id),
            row.tournament_id,
            "organizer",
            diagnostics,
        );
    }

    keys.clear();
    for row in &source.invitations {
        duplicate_association(
            &mut keys,
            (row.tournament_id, row.invitee_id),
            Some(row.tournament_id),
            "duplicate_invitation",
            "invitation",
            diagnostics,
        );
        require_tournament(
            tournament_ids.contains(&row.tournament_id),
            row.tournament_id,
            "invitation",
            diagnostics,
        );

        if row.declined_at.is_some() {
            diagnostics.push(AuditDiagnostic::hard_failure(
                Some(row.tournament_id),
                "unexpected_legacy_decline",
                "the pre-final invitation schema cannot contain declined_at",
            ));
        }
    }

    for tournament in &source.tournaments {
        if let Some(series_id) = tournament.series {
            if !series_ids.contains(&series_id) {
                diagnostics.push(AuditDiagnostic::hard_failure(
                    Some(tournament.id),
                    "missing_series",
                    format!("tournament references missing series {series_id}"),
                ));
            }
        }
        if tournament.seats < 1
            || tournament.min_seats < 1
            || tournament.min_seats > tournament.seats
        {
            diagnostics.push(AuditDiagnostic::hard_failure(
                Some(tournament.id),
                "invalid_legacy_seats",
                format!(
                    "invalid seat range min={} max={}",
                    tournament.min_seats, tournament.seats
                ),
            ));
        }
        if let (Some(lower), Some(upper)) = (tournament.band_lower, tournament.band_upper) {
            if lower > upper {
                diagnostics.push(AuditDiagnostic::hard_failure(
                    Some(tournament.id),
                    "invalid_legacy_rating_band",
                    format!("rating lower bound {lower} exceeds upper bound {upper}"),
                ));
            }
        }
    }
}

fn validate_game_relations(
    source: &LegacySource,
    tournament_ids: &HashSet<Uuid>,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    for game in &source.games {
        let Some(tournament_id) = game.tournament_id else {
            diagnostics.push(AuditDiagnostic::hard_failure(
                None,
                "unexpected_unowned_game",
                format!("source included non-tournament game {}", game.id),
            ));
            continue;
        };
        require_tournament(
            tournament_ids.contains(&tournament_id),
            tournament_id,
            &format!("game {}", game.id),
            diagnostics,
        );

        if game.white_id == game.black_id {
            diagnostics.push(AuditDiagnostic::hard_failure(
                Some(tournament_id),
                "same_player_game",
                format!("game {} has one player on both sides", game.id),
            ));
        }
        if game.current_player_id != game.white_id && game.current_player_id != game.black_id {
            diagnostics.push(AuditDiagnostic::hard_failure(
                Some(tournament_id),
                "invalid_current_player",
                format!("game {} has an unrelated current player", game.id),
            ));
        }
        if game.updated_at < game.created_at {
            diagnostics.push(AuditDiagnostic::hard_failure(
                Some(tournament_id),
                "game_time_reversal",
                format!("game {} was updated before it was created", game.id),
            ));
        }
    }
}

fn validate_schedules(
    source: &LegacySource,
    tournament_ids: &HashSet<Uuid>,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    let games_by_id = source
        .games
        .iter()
        .map(|game| (game.id, game))
        .collect::<HashMap<_, _>>();
    let mut schedule_ids = HashSet::new();
    for row in &source.schedules {
        if !schedule_ids.insert(row.id) {
            diagnostics.push(AuditDiagnostic::warning(
                Some(row.tournament_id),
                "duplicate_schedule",
                format!("schedule {} is duplicated", row.id),
            ));
        }
        if !tournament_ids.contains(&row.tournament_id) {
            diagnostics.push(AuditDiagnostic::warning(
                Some(row.tournament_id),
                "missing_schedule_tournament",
                format!(
                    "schedule {} references missing tournament {} and will be dropped",
                    row.id, row.tournament_id
                ),
            ));
        }
        let Some(game) = games_by_id.get(&row.game_id) else {
            diagnostics.push(AuditDiagnostic::warning(
                Some(row.tournament_id),
                "missing_schedule_game",
                format!(
                    "schedule {} references missing game {}",
                    row.id, row.game_id
                ),
            ));
            continue;
        };
        if game.tournament_id != Some(row.tournament_id) {
            diagnostics.push(AuditDiagnostic::warning(
                Some(row.tournament_id),
                "schedule_tournament_mismatch",
                format!(
                    "schedule {} and game {} disagree on tournament",
                    row.id, row.game_id
                ),
            ));
        }
        let players = BTreeSet::from([game.white_id, game.black_id]);
        if BTreeSet::from([row.proposer_id, row.opponent_id]) != players {
            diagnostics.push(AuditDiagnostic::warning(
                Some(row.tournament_id),
                "schedule_player_mismatch",
                format!(
                    "schedule {} players disagree with game {}",
                    row.id, row.game_id
                ),
            ));
        }
    }
}

fn validate_series(
    source: &LegacySource,
    series_ids: &HashSet<Uuid>,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    let mut keys = HashSet::new();
    for row in &source.series_organizers {
        if !keys.insert((row.series_id, row.organizer_id)) {
            diagnostics.push(AuditDiagnostic::hard_failure(
                None,
                "duplicate_series_organizer",
                format!(
                    "series organizer {} / {} is duplicated",
                    row.series_id, row.organizer_id
                ),
            ));
        }
        if !series_ids.contains(&row.series_id) {
            diagnostics.push(AuditDiagnostic::hard_failure(
                None,
                "missing_organizer_series",
                format!(
                    "series organizer references missing series {}",
                    row.series_id
                ),
            ));
        }
    }
}

fn unique_uuid_ids(
    ids: impl Iterator<Item = Uuid>,
    code: &str,
    label: &str,
    diagnostics: &mut Vec<AuditDiagnostic>,
) -> HashSet<Uuid> {
    let mut unique = HashSet::new();
    for id in ids {
        if !unique.insert(id) {
            diagnostics.push(AuditDiagnostic::hard_failure(
                None,
                code,
                format!("{label} {id} is duplicated"),
            ));
        }
    }
    unique
}

fn duplicate_association<K: Eq + Hash + Copy>(
    keys: &mut HashSet<K>,
    key: K,
    tournament_id: Option<Uuid>,
    code: &str,
    label: &str,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    if !keys.insert(key) {
        diagnostics.push(AuditDiagnostic::hard_failure(
            tournament_id,
            code,
            format!("legacy {label} association is duplicated"),
        ));
    }
}

fn require_tournament(
    found: bool,
    tournament_id: Uuid,
    relation: &str,
    diagnostics: &mut Vec<AuditDiagnostic>,
) {
    if !found {
        diagnostics.push(AuditDiagnostic::hard_failure(
            Some(tournament_id),
            "missing_referenced_tournament",
            format!("{relation} references missing tournament {tournament_id}"),
        ));
    }
}

fn tournament_report(
    source: &LegacySource,
    tournament_id: Uuid,
    intended_mapping: Option<SupportedLegacyMapping>,
    plan: Option<&TournamentPlan>,
) -> TournamentAuditReport {
    let tournament = source
        .tournaments
        .iter()
        .find(|row| row.id == tournament_id)
        .expect("report tournament came from the source");
    let memberships = source
        .memberships
        .iter()
        .filter(|row| row.tournament_id == tournament_id)
        .count();
    let organizers = source
        .organizers
        .iter()
        .filter(|row| row.tournament_id == tournament_id)
        .count();
    let invitations = source
        .invitations
        .iter()
        .filter(|row| row.tournament_id == tournament_id)
        .count();
    let games = source
        .games
        .iter()
        .filter(|row| row.tournament_id == Some(tournament_id))
        .collect::<Vec<_>>();
    let schedules = source
        .schedules
        .iter()
        .filter(|row| row.tournament_id == tournament_id)
        .count();
    let recognized_committee_results = games
        .iter()
        .filter(|game| game.conclusion == "Committee")
        .count();
    let (rounds, pairings, byes_or_sit_outs) = plan.map_or((0, 0, 0), plan_shape_counts);

    TournamentAuditReport {
        tournament_id,
        legacy_mode: tournament.mode.clone(),
        legacy_status: tournament.status.clone(),
        legacy_configuration: LegacyConfigurationReport {
            scoring: tournament.scoring.clone(),
            tiebreaker: tournament.tiebreaker.clone(),
            seats: tournament.seats,
            min_seats: tournament.min_seats,
            rounds: tournament.rounds,
            invite_only: tournament.invite_only,
            time_mode: tournament.time_mode.clone(),
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            band_upper: tournament.band_upper,
            band_lower: tournament.band_lower,
            start_mode: tournament.start_mode.clone(),
            starts_at: tournament.starts_at,
            ends_at: tournament.ends_at,
            round_duration: tournament.round_duration,
        },
        intended_mapping,
        mapped_starts_at: plan.and_then(|plan| plan.tournament.starts_at),
        mapped_configuration: plan.map(|plan| plan.configuration.clone()),
        participants: count(memberships),
        organizers: count(organizers),
        invitations: count(invitations),
        declined_invitations: count(
            source
                .invitations
                .iter()
                .filter(|row| row.tournament_id == tournament_id && row.declined_at.is_some())
                .count(),
        ),
        series: tournament.series,
        games: count(games.len()),
        schedules: count(schedules),
        rounds,
        pairings,
        byes_or_sit_outs,
        recognized_committee_results: count(recognized_committee_results),
    }
}

fn plan_shape_counts(plan: &TournamentPlan) -> (u64, u64, u64) {
    match plan.mapping {
        SupportedLegacyMapping::RoundRobin => {
            let games_per_round = plan
                .roster
                .as_ref()
                .map_or(0, |roster| roster.participants.len() / 2);
            let rounds = plan.slots.len().checked_div(games_per_round).unwrap_or(0);
            let sit_outs = if plan
                .roster
                .as_ref()
                .is_some_and(|roster| roster.participants.len() % 2 == 1)
            {
                rounds
            } else {
                0
            };
            (count(rounds), count(plan.slots.len()), count(sit_outs))
        }
        SupportedLegacyMapping::FinishedDoubleSwiss => {
            let accepted = &plan.accepted_swiss_rounds;
            let pairings = accepted.iter().map(|round| round.pairings.len()).sum();
            (count(accepted.len()), count(pairings), 0)
        }
    }
}

fn audit_counts(
    source: &LegacySource,
    reports: &[TournamentAuditReport],
    plans: &[TournamentPlan],
    diagnostics: &[AuditDiagnostic],
) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for (name, value) in [
        ("tournaments", source.tournaments.len()),
        ("memberships", source.memberships.len()),
        ("organizers", source.organizers.len()),
        ("invitations", source.invitations.len()),
        ("games", source.games.len()),
        ("schedules", source.schedules.len()),
        ("series", source.series.len()),
        ("series_organizers", source.series_organizers.len()),
        ("audited_tournaments", reports.len()),
        ("mapped_tournaments", plans.len()),
    ] {
        counts.insert(name.to_string(), count(value));
    }
    counts.insert(
        String::from("warnings"),
        count(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == AuditSeverity::Warning)
                .count(),
        ),
    );
    counts.insert(
        String::from("hard_failures"),
        count(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == AuditSeverity::HardFailure)
                .count(),
        ),
    );
    counts
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_tournaments::model::{
        LegacyScheduleRow,
        LegacySourceMetadata,
        LegacyTournamentRow,
    };
    use chrono::{TimeZone, Utc};

    fn not_started_tournament(id: Uuid) -> LegacyTournamentRow {
        LegacyTournamentRow {
            id,
            nanoid: format!("tournament-{}", id.as_u128()),
            name: String::from("Legacy"),
            description: String::new(),
            scoring: String::from("Game"),
            tiebreaker: vec![Some(String::from("RawPoints"))],
            seats: 4,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: String::from("DoubleRoundRobin"),
            time_mode: String::from("Real Time"),
            time_base: Some(180),
            time_increment: Some(2),
            band_upper: None,
            band_lower: None,
            start_mode: String::from("Manual"),
            starts_at: None,
            ends_at: None,
            started_at: None,
            round_duration: None,
            status: String::from("NotStarted"),
            created_at: Utc.timestamp_opt(1, 0).unwrap(),
            updated_at: Utc.timestamp_opt(2, 0).unwrap(),
            series: None,
        }
    }

    fn empty_source(tournaments: Vec<LegacyTournamentRow>) -> LegacySource {
        LegacySource {
            metadata: LegacySourceMetadata {
                database_name: String::from("hive-test"),
            },
            tournaments,
            memberships: Vec::new(),
            organizers: Vec::new(),
            invitations: Vec::new(),
            games: Vec::new(),
            schedules: Vec::new(),
            series: Vec::new(),
            series_organizers: Vec::new(),
        }
    }

    #[test]
    fn shared_audit_sorts_plans_and_unknown_modes_fail_closed() {
        let mut second = not_started_tournament(Uuid::from_u128(2));
        let first = not_started_tournament(Uuid::from_u128(1));
        second.mode = String::from("SingleElimination");
        let outcome = run(&empty_source(vec![second, first]));

        assert!(outcome.has_hard_failures());
        assert_eq!(
            outcome
                .plans
                .iter()
                .map(|plan| plan.tournament.id)
                .collect::<Vec<_>>(),
            vec![Uuid::from_u128(1)]
        );
        assert!(outcome.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsupported_legacy_mode"
                && diagnostic.tournament_id == Some(Uuid::from_u128(2))
        }));
    }

    #[test]
    fn audit_exposes_mapped_start_separately_from_rule_configuration() {
        let starts_at = Utc.timestamp_opt(10, 0).unwrap();
        let mut tournament = not_started_tournament(Uuid::from_u128(1));
        tournament.start_mode = String::from("Date");
        tournament.starts_at = Some(starts_at);
        let outcome = run(&empty_source(vec![tournament]));

        assert!(!outcome.has_hard_failures());
        let report = &outcome.report.tournaments[0];
        assert_eq!(report.mapped_starts_at, Some(starts_at));
        let configuration =
            serde_json::to_value(report.mapped_configuration.as_ref().unwrap()).unwrap();
        assert!(configuration.get("start_policy").is_none());
    }

    #[test]
    fn malformed_schedule_metadata_warns_without_blocking_tournament_mapping() {
        let tournament = not_started_tournament(Uuid::from_u128(1));
        let mut source = empty_source(vec![tournament.clone()]);
        source.schedules.push(LegacyScheduleRow {
            id: Uuid::from_u128(2),
            game_id: Uuid::from_u128(3),
            tournament_id: tournament.id,
            proposer_id: Uuid::from_u128(4),
            opponent_id: Uuid::from_u128(5),
            starts_at: Utc.timestamp_opt(10, 0).unwrap(),
            agreed: false,
            notified: false,
        });

        let outcome = run(&source);
        assert!(!outcome.has_hard_failures());
        assert_eq!(outcome.plans.len(), 1);
        assert!(outcome
            .report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing_schedule_game"
                && diagnostic.severity == AuditSeverity::Warning));
    }
}
