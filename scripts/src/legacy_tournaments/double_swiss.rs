use super::{
    legacy_standings::{LegacyStandings, LegacyTiebreaker},
    model::{
        AcceptedRatingPlan,
        AcceptedSwissRoundPlan,
        AuditDiagnostic,
        FinalOutcomePlan,
        LegacyGameRow,
        LegacyMembershipRow,
        LegacyRankGroup,
        LegacyTournamentRow,
        Participant,
        Roster,
        SlotPlan,
        SlotPlanStatus,
        SupportedLegacyMapping,
        SwissPairingPlan,
        TournamentPlan,
    },
};
use db_lib::tournaments::build_swiss_config;
use hive_lib::Color;
use shared_types::{
    tournament::{
        standings::{Group, Placement, Row, Snapshot, Value as StandingValue},
        swiss::{
            Acceleration,
            Config as SwissConfig,
            Criterion as SwissCriterion,
            DirectEncounterOptions,
            DoubleSwissConfig,
            PrimaryScore,
            RoundConfiguration,
            ScoreBasis,
            SonnebornBergerOptions,
            System as SwissSystem,
        },
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        BotAdmission,
        Clock,
        Config,
        DirectEncounterForfeitPolicy,
        FormatConfig,
        GameOutcome as Outcome,
        MatchPointSystem,
        PlayedGameOutcome as PlayedOutcome,
        PointSystem,
        ReleasePolicy,
        RepeatedEncounterPolicy,
        SlotKey,
        SwissGameId,
        SwissLeg,
    },
    TimeMode,
    TournamentGameResult,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroU32,
    str::FromStr,
};
use tournamint::{
    swiss::{
        project as project_swiss,
        DoubleSwissByePoints,
        RoundPairings,
        SwissEncounterFact,
        SwissFacts,
        SwissProjection,
        SwissRoundFact,
        UnplayedRoundPolicy,
    },
    Pairing,
    PlayerId,
};
use uuid::Uuid;

/// All rows passed to this mapper must already belong to `tournament`.
/// Keeping the boundary explicit lets the audit layer group the bulk-loaded
/// legacy source once without coupling this pure reconstruction to SQL.
pub struct DoubleSwissInput<'a> {
    pub tournament: &'a LegacyTournamentRow,
    pub memberships: &'a [LegacyMembershipRow],
    pub games: &'a [LegacyGameRow],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegacyResult {
    WhiteWin,
    Draw,
    BlackWin,
    DoubleForfeit,
}

#[derive(Clone, Copy)]
struct NormalizedGame<'a> {
    row: &'a LegacyGameRow,
    result: LegacyResult,
    outcome: Outcome,
}

#[derive(Clone, Copy)]
struct HistoricalPairing<'a> {
    first: NormalizedGame<'a>,
    second: NormalizedGame<'a>,
}

pub fn map_double_swiss(
    input: &DoubleSwissInput<'_>,
) -> Result<TournamentPlan, Vec<AuditDiagnostic>> {
    map_double_swiss_inner(input).map_err(|diagnostic| vec![diagnostic])
}

fn map_double_swiss_inner(input: &DoubleSwissInput<'_>) -> Result<TournamentPlan, AuditDiagnostic> {
    let tournament = input.tournament;
    ensure(
        tournament.mode == "DoubleSwiss",
        tournament,
        "legacy_double_swiss_wrong_mode",
        format!(
            "Double-Swiss mapper received legacy mode {:?}",
            tournament.mode
        ),
    )?;
    ensure(
        tournament.status == "Finished",
        tournament,
        "legacy_double_swiss_nonfinished",
        format!(
            "legacy Double-Swiss status {:?} is unsupported; only Finished was deliberately mapped",
            tournament.status
        ),
    )?;
    ensure(
        tournament.started_at.is_some(),
        tournament,
        "legacy_double_swiss_missing_start",
        "a finished legacy Double-Swiss tournament has no started_at timestamp",
    )?;
    // The retired implementation always ranked Double-Swiss by individual
    // game points. No production-shaped Match-scored example exists, and the
    // old display/pairing code did not implement alternate match semantics.
    ensure(
        tournament.scoring == "Game",
        tournament,
        "legacy_double_swiss_unknown_scoring",
        format!(
            "legacy Double-Swiss scoring {:?} has no deliberate conversion",
            tournament.scoring
        ),
    )?;

    let clock = legacy_clock(tournament)?;
    let criteria = legacy_criteria(tournament)?;
    let roster = frozen_roster(input)?;
    let players = (0..roster.participants.len())
        .map(PlayerId::new)
        .collect::<Vec<_>>();

    let normalized_games = normalize_games(input)?;
    let historical_rounds =
        reconstruct_rounds(tournament, &roster.participants, &normalized_games)?;
    let round_count = u32::try_from(historical_rounds.len())
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| {
            failure(
                tournament,
                "legacy_double_swiss_round_count_out_of_range",
                "the reconstructed Double-Swiss round count does not fit a non-zero u32",
            )
        })?;

    let configuration = double_swiss_configuration(clock, round_count, &criteria);
    let FormatConfig::Swiss(swiss_config) = &configuration.format else {
        unreachable!("the local builder constructs a Swiss configuration")
    };
    let native_config = build_swiss_config(swiss_config);
    let player_by_user = player_map(&roster.participants, &players);
    let mut facts = SwissFacts {
        config: native_config,
        initial_ranking: players,
        rounds: Vec::with_capacity(historical_rounds.len()),
    };

    let mut slots = Vec::with_capacity(normalized_games.len());
    let mut accepted_swiss_rounds = Vec::with_capacity(historical_rounds.len());
    let mut slot_by_game = HashMap::with_capacity(normalized_games.len());

    for (round_index, historical_pairings) in historical_rounds.iter().enumerate() {
        let pairings = historical_round_pairings(&player_by_user, historical_pairings);
        let round_id = u32::try_from(round_index).map_err(|_| {
            failure(
                tournament,
                "legacy_double_swiss_round_identity_out_of_range",
                "reconstructed round ID is out of range",
            )
        })?;
        let persisted_pairings = pairings
            .iter()
            .map(|pairing| SwissPairingPlan {
                white_id: pairing.first.row.white_id,
                black_id: pairing.first.row.black_id,
            })
            .collect();
        let accepted_ratings =
            historical_round_rating_snapshots(tournament, &pairings, round_index)?;
        accepted_swiss_rounds.push(AcceptedSwissRoundPlan {
            round_index: round_id,
            pairings: persisted_pairings,
            accepted_ratings,
        });

        for (pairing_index, pairing) in pairings.iter().enumerate() {
            let pairing_index = u32::try_from(pairing_index).map_err(|_| {
                failure(
                    tournament,
                    "legacy_double_swiss_pairing_identity_out_of_range",
                    "reconstructed pairing ordinal is out of range",
                )
            })?;
            for (leg, game) in [
                (SwissLeg::First, pairing.first),
                (SwissLeg::Second, pairing.second),
            ] {
                let key = SlotKey::Swiss {
                    slot: SwissGameId {
                        round_index: round_id,
                        pairing_index,
                        leg,
                    },
                };
                if slot_by_game.insert(game.row.id, key).is_some() {
                    return Err(failure(
                        tournament,
                        "legacy_double_swiss_duplicate_game_binding",
                        format!("game {} was bound more than once", game.row.id),
                    ));
                }
                slots.push(SlotPlan {
                    key,
                    white_id: game.row.white_id,
                    black_id: game.row.black_id,
                    clock,
                    status: SlotPlanStatus::Sealed {
                        game_id: game.row.id,
                        outcome: game.outcome,
                    },
                });
            }
        }
        let encounters: Vec<SwissEncounterFact> = pairings
            .iter()
            .map(|pairing| SwissEncounterFact {
                pairing: Pairing::new(
                    player_by_user[&pairing.first.row.white_id],
                    player_by_user[&pairing.first.row.black_id],
                ),
                outcomes: vec![Some(pairing.first.outcome), Some(pairing.second.outcome)],
            })
            .collect();
        facts.rounds.push(SwissRoundFact {
            eligible: facts.initial_ranking.clone(),
            pairings: RoundPairings {
                byes: Vec::new(),
                games: encounters
                    .iter()
                    .map(|encounter| encounter.pairing)
                    .collect(),
            },
            encounters,
        });
    }

    ensure(
        slots.len() == normalized_games.len()
            && accepted_swiss_rounds.len() == historical_rounds.len()
            && slot_by_game.len() == normalized_games.len(),
        tournament,
        "legacy_double_swiss_incomplete_consumption",
        "not every historical game/result was consumed exactly once",
    )?;
    let expected_rank_groups = legacy_rank_groups(
        tournament,
        &roster.participants,
        &normalized_games,
        &criteria,
    )?;
    let projection = project_swiss(&facts).map_err(|error| {
        failure(
            tournament,
            "legacy_double_swiss_standings_failed",
            format!("mapped Double-Swiss standings failed: {error}"),
        )
    })?;
    let actual_rank_groups = engine_rank_groups(tournament, &projection, &roster.participants)?;
    ensure(
        actual_rank_groups == expected_rank_groups,
        tournament,
        "legacy_double_swiss_rank_mismatch",
        format!(
            "Tournamint rank groups {:?} do not match frozen legacy groups {:?}",
            actual_rank_groups, expected_rank_groups
        ),
    )?;

    let final_outcome = swiss_final_outcome(tournament, swiss_config, &projection, &roster)?;

    Ok(TournamentPlan {
        tournament: tournament.clone(),
        mapping: SupportedLegacyMapping::FinishedDoubleSwiss,
        configuration,
        roster: Some(roster),
        slots,
        accepted_swiss_rounds,
        expected_rank_groups,
        // Canonical slots, accepted rounds, and the final outcome are frozen
        // together so the importer has no replay-only dependency.
        final_outcome: Some(final_outcome),
    })
}

fn legacy_clock(tournament: &LegacyTournamentRow) -> Result<Clock, AuditDiagnostic> {
    let mode = TimeMode::from_str(&tournament.time_mode).map_err(|error| {
        failure(
            tournament,
            "legacy_double_swiss_unknown_time_mode",
            format!("could not parse legacy time mode: {error}"),
        )
    })?;
    Clock::from_time_parts(mode, tournament.time_base, tournament.time_increment)
        .map_err(|error| {
            failure(
                tournament,
                "legacy_double_swiss_invalid_clock",
                error.to_string(),
            )
        })?
        .ok_or_else(|| {
            failure(
                tournament,
                "legacy_double_swiss_untimed",
                "the typed Swiss model has no untimed clock mapping",
            )
        })
}

fn legacy_criteria(
    tournament: &LegacyTournamentRow,
) -> Result<Vec<LegacyTiebreaker>, AuditDiagnostic> {
    let mut criteria = vec![LegacyTiebreaker::RawPoints];
    for stored in &tournament.tiebreaker {
        let Some(stored) = stored.as_deref() else {
            return Err(failure(
                tournament,
                "legacy_double_swiss_null_tiebreaker",
                "legacy tiebreaker array contains NULL",
            ));
        };
        let criterion = LegacyTiebreaker::from_str(stored).map_err(|error| {
            failure(tournament, "legacy_double_swiss_unknown_tiebreaker", error)
        })?;
        if !criteria.contains(&criterion) {
            criteria.push(criterion);
        }
    }
    Ok(criteria)
}

fn double_swiss_configuration(
    clock: Clock,
    rounds: NonZeroU32,
    criteria: &[LegacyTiebreaker],
) -> Config {
    let match_points = MatchPointSystem::STANDARD;
    let standings = criteria
        .iter()
        .map(|criterion| match criterion {
            LegacyTiebreaker::RawPoints => SwissCriterion::PrimaryScore,
            LegacyTiebreaker::HeadToHead => {
                SwissCriterion::DirectEncounter(DirectEncounterOptions {
                    score_basis: ScoreBasis::Primary,
                    forfeits: DirectEncounterForfeitPolicy::IncludeAsScored,
                    repeated_encounters: RepeatedEncounterPolicy::Sum,
                })
            }
            LegacyTiebreaker::WinsAsBlack => SwissCriterion::GamesWonWithBlack,
            LegacyTiebreaker::SonnebornBerger => {
                SwissCriterion::SonnebornBerger(SonnebornBergerOptions {
                    score_basis: ScoreBasis::Primary,
                    cut_lowest: 0,
                    unplayed: UnplayedRoundPolicy::FideMarch2026,
                })
            }
        })
        .collect();
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(SwissConfig {
            system: SwissSystem::DoubleSwiss(DoubleSwissConfig {
                primary_score: PrimaryScore::GamePoints,
                match_point_system: match_points,
                bye_points: DoubleSwissByePoints::standard(PointSystem::STANDARD, match_points)
                    .expect("standard point systems cannot overflow"),
            }),
            double_swiss_release_policy: ReleasePolicy::FullyUnlocked,
            rounds: RoundConfiguration::resolved(rounds, None),
            clock,
            game_point_system: PointSystem::STANDARD,
            acceleration: Acceleration::None,
            standings,
        }),
    }
}

fn frozen_roster(input: &DoubleSwissInput<'_>) -> Result<Roster, AuditDiagnostic> {
    let tournament = input.tournament;
    let mut user_ids = Vec::with_capacity(input.memberships.len());
    for membership in input.memberships {
        ensure(
            membership.tournament_id == tournament.id,
            tournament,
            "legacy_double_swiss_foreign_membership",
            format!(
                "membership for tournament {} was passed to tournament {}",
                membership.tournament_id, tournament.id
            ),
        )?;
        user_ids.push(membership.user_id);
    }
    user_ids.sort_unstable();
    let original_len = user_ids.len();
    user_ids.dedup();
    ensure(
        user_ids.len() == original_len,
        tournament,
        "legacy_double_swiss_duplicate_membership",
        "the legacy Double-Swiss roster contains a duplicate member",
    )?;
    ensure(
        user_ids.len() >= 2,
        tournament,
        "legacy_double_swiss_roster_too_small",
        "a finished legacy Double-Swiss tournament needs at least two members",
    )?;
    ensure(
        user_ids.len().is_multiple_of(2),
        tournament,
        "legacy_double_swiss_odd_roster",
        "the complete historical roster is odd; the persisted SwissByePlayer membership is missing",
    )?;

    let participants = user_ids
        .iter()
        .enumerate()
        .map(|(index, user_id)| {
            Ok(Participant {
                user_id: *user_id,
                pairing_number: u32::try_from(index + 1).map_err(|_| {
                    failure(
                        tournament,
                        "legacy_double_swiss_roster_too_large",
                        "roster pairing number is out of range",
                    )
                })?,
            })
        })
        .collect::<Result<Vec<_>, AuditDiagnostic>>()?;
    Ok(Roster { participants })
}

fn normalize_games<'a>(
    input: &'a DoubleSwissInput<'a>,
) -> Result<Vec<NormalizedGame<'a>>, AuditDiagnostic> {
    let tournament = input.tournament;
    ensure(
        !input.games.is_empty(),
        tournament,
        "legacy_double_swiss_missing_games",
        "a finished legacy Double-Swiss tournament has no games",
    )?;
    let mut rows = input.games.iter().collect::<Vec<_>>();
    rows.sort_by_key(|game| (game.created_at, game.id));
    let mut game_ids = HashSet::with_capacity(rows.len());
    let mut normalized = Vec::with_capacity(rows.len());
    for game in rows {
        ensure(
            game.tournament_id == Some(tournament.id),
            tournament,
            "legacy_double_swiss_foreign_game",
            format!(
                "game {} has tournament ownership {:?}, expected {}",
                game.id, game.tournament_id, tournament.id
            ),
        )?;
        ensure(
            game_ids.insert(game.id),
            tournament,
            "legacy_double_swiss_duplicate_game",
            format!("game {} occurs more than once", game.id),
        )?;
        ensure(
            game.time_mode == tournament.time_mode
                && game.time_base == tournament.time_base
                && game.time_increment == tournament.time_increment,
            tournament,
            "legacy_double_swiss_game_clock_mismatch",
            format!("game {} does not use the tournament clock", game.id),
        )?;
        let (result, outcome) = normalize_game(tournament, game)?;
        normalized.push(NormalizedGame {
            row: game,
            result,
            outcome,
        });
    }
    Ok(normalized)
}

fn normalize_game(
    tournament: &LegacyTournamentRow,
    game: &LegacyGameRow,
) -> Result<(LegacyResult, Outcome), AuditDiagnostic> {
    ensure(
        game.finished,
        tournament,
        "legacy_double_swiss_unfinished_game",
        format!("game {} is unfinished", game.id),
    )?;
    let parsed_result =
        TournamentGameResult::from_str(&game.tournament_game_result).map_err(|_| {
            failure(
                tournament,
                "legacy_double_swiss_unknown_result",
                format!(
                    "game {} has unknown tournament result {:?}",
                    game.id, game.tournament_game_result
                ),
            )
        })?;
    let result = match parsed_result {
        TournamentGameResult::Winner(Color::White) => LegacyResult::WhiteWin,
        TournamentGameResult::Draw => LegacyResult::Draw,
        TournamentGameResult::Winner(Color::Black) => LegacyResult::BlackWin,
        TournamentGameResult::DoubleForfeit => LegacyResult::DoubleForfeit,
        TournamentGameResult::Unknown => {
            return Err(failure(
                tournament,
                "legacy_double_swiss_unknown_result",
                format!("game {} has no terminal result", game.id),
            ));
        }
    };

    if game.game_status == "Adjudicated" && game.conclusion == "Committee" {
        ensure_exact_adjudicated_shape(tournament, game, "Committee")?;
        let played = match result {
            LegacyResult::WhiteWin => PlayedOutcome::WhiteWin,
            LegacyResult::Draw => PlayedOutcome::Draw,
            LegacyResult::BlackWin => PlayedOutcome::BlackWin,
            LegacyResult::DoubleForfeit => {
                return Err(failure(
                    tournament,
                    "legacy_double_swiss_unknown_committee_shape",
                    format!(
                        "Committee game {} has unsupported double-forfeit result",
                        game.id
                    ),
                ));
            }
        };
        // This is the one intentional legacy exception: these administrative
        // rows represented played results in the retired standings, including
        // both SwissByePlayer and human-human decisions.
        return Ok((result, Outcome::Played(played)));
    }

    if game.game_status == "Adjudicated" && game.conclusion == "Forfeit" {
        ensure_exact_adjudicated_shape(tournament, game, "Forfeit")?;
        ensure(
            result == LegacyResult::DoubleForfeit,
            tournament,
            "legacy_double_swiss_unknown_forfeit_shape",
            format!(
                "Forfeit game {} is not the legacy double-forfeit shape",
                game.id
            ),
        )?;
        return Ok((
            result,
            Outcome::Adjudicated(
                AdjudicatedGameOutcome::new(
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit,
                )
                .expect("valid double forfeit"),
            ),
        ));
    }

    let expected_status = match result {
        LegacyResult::WhiteWin => "Finished(1-0)",
        LegacyResult::Draw => "Finished(½-½)",
        LegacyResult::BlackWin => "Finished(0-1)",
        LegacyResult::DoubleForfeit => {
            return Err(failure(
                tournament,
                "legacy_double_swiss_unknown_terminal_shape",
                format!(
                    "double-forfeit game {} is not an exact adjudicated Forfeit row",
                    game.id
                ),
            ));
        }
    };
    ensure(
        game.game_status == expected_status,
        tournament,
        "legacy_double_swiss_result_status_mismatch",
        format!(
            "game {} status {:?} disagrees with result {:?}",
            game.id, game.game_status, game.tournament_game_result
        ),
    )?;
    if !matches!(
        (game.conclusion.as_str(), result),
        ("Board", _)
            | (
                "Resigned" | "Withdrawal",
                LegacyResult::WhiteWin | LegacyResult::BlackWin
            )
            | ("Draw", LegacyResult::Draw)
            | ("Timeout", LegacyResult::WhiteWin | LegacyResult::BlackWin)
            | ("Repetition", LegacyResult::Draw)
    ) {
        return Err(failure(
            tournament,
            "legacy_double_swiss_unknown_terminal_shape",
            format!(
                "game {} has unsupported conclusion/result pair {:?}/{:?}",
                game.id, game.conclusion, game.tournament_game_result
            ),
        ));
    }
    let outcome = Outcome::Played(match result {
        LegacyResult::WhiteWin => PlayedOutcome::WhiteWin,
        LegacyResult::Draw => PlayedOutcome::Draw,
        LegacyResult::BlackWin => PlayedOutcome::BlackWin,
        LegacyResult::DoubleForfeit => unreachable!("handled above"),
    });
    Ok((result, outcome))
}

fn ensure_exact_adjudicated_shape(
    tournament: &LegacyTournamentRow,
    game: &LegacyGameRow,
    kind: &str,
) -> Result<(), AuditDiagnostic> {
    ensure(
        game.turn == 0
            && game.history.is_empty()
            && game.game_control_history.is_empty()
            && game.hashes.is_empty()
            && game.move_times.is_empty()
            && game.game_start == "Ready",
        tournament,
        if kind == "Committee" {
            "legacy_double_swiss_unknown_committee_shape"
        } else {
            "legacy_double_swiss_unknown_forfeit_shape"
        },
        format!(
            "{kind} game {} is not an untouched legacy adjudication row",
            game.id
        ),
    )
}

fn reconstruct_rounds<'a>(
    tournament: &LegacyTournamentRow,
    roster: &[Participant],
    games: &[NormalizedGame<'a>],
) -> Result<Vec<Vec<HistoricalPairing<'a>>>, AuditDiagnostic> {
    let round_size = roster.len();
    ensure(
        games.len().is_multiple_of(round_size),
        tournament,
        "legacy_double_swiss_incomplete_round_batch",
        format!(
            "{} games cannot be partitioned into complete {}-game rounds",
            games.len(),
            round_size
        ),
    )?;
    let roster_users = roster
        .iter()
        .map(|participant| participant.user_id)
        .collect::<BTreeSet<_>>();
    let mut rounds = Vec::with_capacity(games.len() / round_size);
    let mut previous_round_finished_at = None;
    for (round_index, chunk) in games.chunks_exact(round_size).enumerate() {
        let round_created_at = chunk
            .iter()
            .map(|game| game.row.created_at)
            .min()
            .expect("chunks_exact yielded a nonempty round");
        if let Some(previous_finished_at) = previous_round_finished_at {
            ensure(
                previous_finished_at <= round_created_at,
                tournament,
                "legacy_double_swiss_overlapping_round_batches",
                format!(
                    "round {} was created before every game in round {} became terminal",
                    round_index + 1,
                    round_index
                ),
            )?;
        }
        previous_round_finished_at = chunk.iter().map(|game| game.row.updated_at).max();
        let mut by_opponents = BTreeMap::<(Uuid, Uuid), Vec<NormalizedGame<'a>>>::new();
        for game in chunk {
            ensure(
                game.row.white_id != game.row.black_id,
                tournament,
                "legacy_double_swiss_self_pairing",
                format!("game {} is a self-pairing", game.row.id),
            )?;
            ensure(
                roster_users.contains(&game.row.white_id)
                    && roster_users.contains(&game.row.black_id),
                tournament,
                "legacy_double_swiss_missing_player",
                format!(
                    "round {} game {} references a user outside the complete roster",
                    round_index + 1,
                    game.row.id
                ),
            )?;
            let key = if game.row.white_id < game.row.black_id {
                (game.row.white_id, game.row.black_id)
            } else {
                (game.row.black_id, game.row.white_id)
            };
            by_opponents.entry(key).or_default().push(*game);
        }

        let mut assigned = BTreeSet::new();
        let mut pairings = Vec::with_capacity(round_size / 2);
        for ((first_user, second_user), mut reciprocal) in by_opponents {
            reciprocal.sort_by_key(|game| (game.row.created_at, game.row.id));
            ensure(
                reciprocal.len() == 2,
                tournament,
                "legacy_double_swiss_nonreciprocal_pairing",
                format!(
                    "round {} pairing {first_user}/{second_user} has {} games instead of two",
                    round_index + 1,
                    reciprocal.len()
                ),
            )?;
            let first = reciprocal[0];
            let second = reciprocal[1];
            ensure(
                first.row.white_id == second.row.black_id
                    && first.row.black_id == second.row.white_id,
                tournament,
                "legacy_double_swiss_nonreciprocal_pairing",
                format!(
                    "round {} pairing {first_user}/{second_user} does not contain reciprocal colors",
                    round_index + 1
                ),
            )?;
            let first_is_committee = first.row.conclusion == "Committee";
            let second_is_committee = second.row.conclusion == "Committee";
            if first_is_committee || second_is_committee {
                let first_winner = winner_id(first);
                let second_winner = winner_id(second);
                ensure(
                    first_is_committee
                        && second_is_committee
                        && first_winner.is_some()
                        && first_winner == second_winner,
                    tournament,
                    "legacy_double_swiss_unknown_committee_shape",
                    format!(
                        "round {} Committee pairing {first_user}/{second_user} is not one consistent 2-0 decision",
                        round_index + 1
                    ),
                )?;
            }
            ensure(
                assigned.insert(first_user) && assigned.insert(second_user),
                tournament,
                "legacy_double_swiss_duplicate_round_participant",
                format!(
                    "round {} assigns a participant to more than one pairing",
                    round_index + 1
                ),
            )?;
            pairings.push(HistoricalPairing { first, second });
        }
        ensure(
            assigned == roster_users && pairings.len() * 2 == round_size,
            tournament,
            "legacy_double_swiss_incomplete_round_roster",
            format!(
                "round {} does not partition the complete ordinary roster exactly once",
                round_index + 1
            ),
        )?;
        pairings.sort_by_key(|pairing| (pairing.first.row.created_at, pairing.first.row.id));
        rounds.push(pairings);
    }
    Ok(rounds)
}

fn winner_id(game: NormalizedGame<'_>) -> Option<Uuid> {
    match game.result {
        LegacyResult::WhiteWin => Some(game.row.white_id),
        LegacyResult::BlackWin => Some(game.row.black_id),
        LegacyResult::Draw | LegacyResult::DoubleForfeit => None,
    }
}

fn historical_round_pairings<'a>(
    player_by_user: &HashMap<Uuid, PlayerId>,
    historical_pairings: &[HistoricalPairing<'a>],
) -> Vec<HistoricalPairing<'a>> {
    historical_pairings
        .iter()
        .map(|historical| {
            if player_by_user[&historical.first.row.white_id]
                < player_by_user[&historical.first.row.black_id]
            {
                *historical
            } else {
                HistoricalPairing {
                    first: historical.second,
                    second: historical.first,
                }
            }
        })
        .collect()
}

fn historical_round_rating_snapshots(
    tournament: &LegacyTournamentRow,
    pairings: &[HistoricalPairing<'_>],
    round_index: usize,
) -> Result<Vec<AcceptedRatingPlan>, AuditDiagnostic> {
    let mut snapshots = BTreeMap::new();
    for pairing in pairings {
        // A legacy game captured both ratings immediately before applying its
        // result. The first-finished leg is therefore the only leg whose pair
        // has not already been changed by the reciprocal game's result.
        let source = [pairing.first, pairing.second]
            .into_iter()
            .min_by_key(|game| (game.row.updated_at, game.row.id))
            .expect("a historical Double-Swiss pairing always has two legs");
        for (user_id, stored_rating) in [
            (source.row.white_id, source.row.white_rating),
            (source.row.black_id, source.row.black_rating),
        ] {
            let stored_rating = stored_rating.ok_or_else(|| {
                failure(
                    tournament,
                    "legacy_double_swiss_missing_rating_snapshot",
                    format!(
                        "round {} first-finished game {} has no stored rating for user {}",
                        round_index + 1,
                        source.row.id,
                        user_id
                    ),
                )
            })?;
            let rating = legacy_rating_snapshot(stored_rating).ok_or_else(|| {
                failure(
                    tournament,
                    "legacy_double_swiss_invalid_rating_snapshot",
                    format!(
                        "round {} first-finished game {} has invalid stored rating {:?} for user {}",
                        round_index + 1,
                        source.row.id,
                        stored_rating,
                        user_id
                    ),
                )
            })?;
            ensure(
                snapshots.insert(user_id, rating).is_none(),
                tournament,
                "legacy_double_swiss_duplicate_round_rating",
                format!(
                    "round {} recovered more than one rating for user {}",
                    round_index + 1,
                    user_id
                ),
            )?;
        }
    }
    Ok(snapshots
        .into_iter()
        .map(|(user_id, rating)| AcceptedRatingPlan { user_id, rating })
        .collect())
}

fn legacy_rating_snapshot(rating: f64) -> Option<u32> {
    if !rating.is_finite() {
        return None;
    }
    let rating = rating.round().max(0.0);
    (rating <= f64::from(i32::MAX)).then_some(rating as u32)
}

fn player_map(roster: &[Participant], players: &[PlayerId]) -> HashMap<Uuid, PlayerId> {
    roster
        .iter()
        .zip(players.iter().copied())
        .map(|(participant, player)| (participant.user_id, player))
        .collect()
}

fn legacy_rank_groups(
    tournament: &LegacyTournamentRow,
    roster: &[Participant],
    games: &[NormalizedGame<'_>],
    criteria: &[LegacyTiebreaker],
) -> Result<Vec<LegacyRankGroup>, AuditDiagnostic> {
    let expected_users = roster
        .iter()
        .map(|participant| participant.user_id)
        .collect::<BTreeSet<_>>();
    let mut standings = LegacyStandings::default();
    for game in games {
        let result = match game.result {
            LegacyResult::WhiteWin => TournamentGameResult::Winner(Color::White),
            LegacyResult::Draw => TournamentGameResult::Draw,
            LegacyResult::BlackWin => TournamentGameResult::Winner(Color::Black),
            LegacyResult::DoubleForfeit => TournamentGameResult::DoubleForfeit,
        };
        standings
            .add_result(game.row.white_id, game.row.black_id, result)
            .map_err(|error| {
                failure(
                    tournament,
                    "legacy_double_swiss_invalid_legacy_standings",
                    format!("could not replay the retired standings calculation: {error}"),
                )
            })?;
    }
    let groups = standings.rank_groups(criteria).map_err(|error| {
        failure(
            tournament,
            "legacy_double_swiss_invalid_legacy_standings",
            format!("could not rank the retired standings: {error}"),
        )
    })?;
    let ranked_users = groups
        .iter()
        .flat_map(|group| group.user_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    ensure(
        ranked_users == expected_users,
        tournament,
        "legacy_double_swiss_incomplete_legacy_standings",
        "retired standings did not cover the complete ordinary roster",
    )?;
    Ok(groups)
}

fn engine_rank_groups(
    tournament: &LegacyTournamentRow,
    standings: &SwissProjection,
    roster: &[Participant],
) -> Result<Vec<LegacyRankGroup>, AuditDiagnostic> {
    standings
        .standings
        .groups
        .iter()
        .map(|group| {
            let mut user_ids = group
                .players
                .iter()
                .map(|row| {
                    roster
                        .get(row.player.index())
                        .map(|participant| participant.user_id)
                        .ok_or_else(|| {
                            failure(
                                tournament,
                                "legacy_double_swiss_standings_unknown_player",
                                "Tournamint standings reference a player outside the frozen roster",
                            )
                        })
                })
                .collect::<Result<Vec<_>, AuditDiagnostic>>()?;
            user_ids.sort_unstable();
            Ok(LegacyRankGroup {
                competition_rank: group.rank,
                user_ids,
            })
        })
        .collect()
}

fn swiss_final_outcome(
    tournament: &LegacyTournamentRow,
    config: &SwissConfig,
    projection: &SwissProjection,
    roster: &Roster,
) -> Result<FinalOutcomePlan, AuditDiagnostic> {
    let invalid = |message: String| {
        AuditDiagnostic::hard_failure(
            Some(tournament.id),
            "legacy_double_swiss_final_outcome_failed",
            message,
        )
    };
    let double_swiss = matches!(config.system, SwissSystem::DoubleSwiss(_));
    let mut groups = Vec::with_capacity(projection.standings.groups.len());
    for group in &projection.standings.groups {
        let mut rows = Vec::with_capacity(group.players.len());
        for standing in &group.players {
            let participant = roster
                .participants
                .get(standing.player.index())
                .ok_or_else(|| {
                    invalid(String::from(
                        "standings reference a player outside the roster",
                    ))
                })?;
            let summary = projection
                .players
                .iter()
                .find(|summary| summary.player == standing.player)
                .ok_or_else(|| invalid(String::from("standings omit a player summary")))?;
            let mut counts = vec![summary.pairing_allocated_byes, summary.requested_byes];
            if double_swiss {
                counts.extend([
                    summary.match_wins,
                    summary.match_draws,
                    summary.match_losses,
                ]);
            }
            rows.push(Row {
                user_id: participant.user_id,
                primary_score: StandingValue::Score(summary.primary_score),
                games_played: summary.games_played,
                matches_played: double_swiss.then_some(summary.matches_played),
                wins: summary.game_wins,
                draws: summary.game_draws,
                losses: summary.game_losses,
                counts,
                values: standing
                    .values
                    .iter()
                    .copied()
                    .map(StandingValue::from)
                    .collect(),
            });
        }
        groups.push(Group {
            placement: Placement::CompetitionRank(group.rank),
            rows,
            separated_by: group.separated_by,
        });
    }
    Ok(FinalOutcomePlan {
        standings: Snapshot { groups },
    })
}

fn ensure(
    condition: bool,
    tournament: &LegacyTournamentRow,
    code: &str,
    message: impl Into<String>,
) -> Result<(), AuditDiagnostic> {
    condition
        .then_some(())
        .ok_or_else(|| failure(tournament, code, message))
}

fn failure(
    tournament: &LegacyTournamentRow,
    code: &str,
    message: impl Into<String>,
) -> AuditDiagnostic {
    AuditDiagnostic::hard_failure(Some(tournament.id), code, message)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn tournament() -> LegacyTournamentRow {
        let created_at = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        LegacyTournamentRow {
            id: uuid(100),
            nanoid: "legacy-ds".to_string(),
            name: "Legacy Double-Swiss".to_string(),
            description: String::new(),
            scoring: "Game".to_string(),
            tiebreaker: vec![
                Some("RawPoints".to_string()),
                Some("HeadToHead".to_string()),
                Some("WinsAsBlack".to_string()),
                Some("SonnebornBerger".to_string()),
            ],
            seats: 8,
            min_seats: 2,
            // The retired column did not mean reconstructed Swiss rounds.
            rounds: 99,
            invite_only: false,
            mode: "DoubleSwiss".to_string(),
            time_mode: "Real Time".to_string(),
            time_base: Some(300),
            time_increment: Some(3),
            band_upper: None,
            band_lower: None,
            start_mode: "Manual".to_string(),
            starts_at: None,
            ends_at: None,
            started_at: Some(created_at),
            round_duration: None,
            status: "Finished".to_string(),
            created_at,
            updated_at: created_at + Duration::hours(1),
            series: None,
        }
    }

    fn memberships(tournament_id: Uuid, players: &[Uuid]) -> Vec<LegacyMembershipRow> {
        players
            .iter()
            .rev()
            .map(|user_id| LegacyMembershipRow {
                tournament_id,
                user_id: *user_id,
            })
            .collect()
    }

    fn game(
        tournament: &LegacyTournamentRow,
        id: u128,
        white_id: Uuid,
        black_id: Uuid,
        created_offset: i64,
        result: &str,
        conclusion: &str,
    ) -> LegacyGameRow {
        let created_at = tournament.created_at + Duration::seconds(created_offset);
        let committee = conclusion == "Committee";
        LegacyGameRow {
            id: uuid(id),
            nanoid: format!("game-{id}"),
            current_player_id: white_id,
            black_id,
            finished: true,
            game_status: if committee {
                "Adjudicated".to_string()
            } else {
                format!("Finished({result})")
            },
            history: if committee {
                String::new()
            } else {
                "wA1;".to_string()
            },
            game_control_history: String::new(),
            turn: if committee { 0 } else { 1 },
            white_id,
            created_at,
            updated_at: created_at + Duration::seconds(20),
            time_mode: tournament.time_mode.clone(),
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            white_rating: Some(1_500.0 + white_id.as_u128() as f64),
            black_rating: Some(1_500.0 + black_id.as_u128() as f64),
            hashes: Vec::new(),
            conclusion: conclusion.to_string(),
            tournament_id: Some(tournament.id),
            tournament_game_result: result.to_string(),
            game_start: "Ready".to_string(),
            move_times: Vec::new(),
        }
    }

    fn reciprocal(
        tournament: &LegacyTournamentRow,
        first_id: u128,
        white: Uuid,
        black: Uuid,
        offset: i64,
        results: (&str, &str),
        committee: bool,
    ) -> [LegacyGameRow; 2] {
        let conclusion = if committee { "Committee" } else { "Board" };
        [
            game(
                tournament, first_id, white, black, offset, results.0, conclusion,
            ),
            game(
                tournament,
                first_id + 1,
                black,
                white,
                offset + 1,
                results.1,
                conclusion,
            ),
        ]
    }

    pub(crate) fn valid_fixture() -> (
        LegacyTournamentRow,
        Vec<LegacyMembershipRow>,
        Vec<LegacyGameRow>,
    ) {
        let tournament = tournament();
        // Player 4 stands in for the ordinary persisted SwissByePlayer member.
        let players = [uuid(1), uuid(2), uuid(3), uuid(4)];
        let memberships = memberships(tournament.id, &players);
        let mut games = Vec::new();
        games.extend(reciprocal(
            &tournament,
            1000,
            players[0],
            players[1],
            1,
            ("1-0", "0-1"),
            false,
        ));
        games.extend(reciprocal(
            &tournament,
            1010,
            players[2],
            players[3],
            3,
            ("1-0", "0-1"),
            true,
        ));
        games.extend(reciprocal(
            &tournament,
            1020,
            players[3],
            players[0],
            100,
            ("½-½", "1-0"),
            false,
        ));
        games.extend(reciprocal(
            &tournament,
            1030,
            players[1],
            players[2],
            102,
            ("1-0", "0-1"),
            false,
        ));
        // Input order must not define history or pairing ordinals.
        games.reverse();
        (tournament, memberships, games)
    }

    fn diagnostic_code(result: Result<TournamentPlan, Vec<AuditDiagnostic>>) -> String {
        result.unwrap_err()[0].code.clone()
    }

    #[test]
    fn reconstructs_recorded_rounds_and_treats_committee_as_played() {
        let (mut tournament, memberships, games) = valid_fixture();
        let starts_at = tournament.created_at - Duration::hours(1);
        tournament.starts_at = Some(starts_at);
        let plan = map_double_swiss(&DoubleSwissInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();

        assert_eq!(plan.mapping, SupportedLegacyMapping::FinishedDoubleSwiss);
        assert_eq!(plan.tournament.starts_at, Some(starts_at));
        assert_eq!(plan.roster.as_ref().unwrap().participants.len(), 4);
        assert_eq!(plan.slots.len(), 8);
        assert_eq!(plan.accepted_swiss_rounds.len(), 2);
        assert!(plan.final_outcome.is_some());
        let committee_game_ids = [uuid(1010), uuid(1011)];
        for slot in plan.slots.iter().filter(|slot| {
            matches!(
                slot.status,
                SlotPlanStatus::Sealed { game_id, .. } if committee_game_ids.contains(&game_id)
            )
        }) {
            assert!(matches!(
                slot.status,
                SlotPlanStatus::Sealed {
                    outcome: Outcome::Played(_),
                    ..
                }
            ));
        }
        let FormatConfig::Swiss(config) = &plan.configuration.format else {
            panic!("expected Swiss configuration")
        };
        assert_eq!(config.rounds.resolved_rounds().unwrap().get(), 2);
    }

    #[test]
    fn recovers_each_round_rating_from_the_first_finished_leg() {
        let (tournament, memberships, mut games) = valid_fixture();
        let first_created = games.iter_mut().find(|game| game.id == uuid(1000)).unwrap();
        first_created.white_rating = Some(1_111.0);
        first_created.black_rating = Some(2_222.0);
        first_created.updated_at = tournament.created_at + Duration::seconds(30);
        let first_finished = games.iter_mut().find(|game| game.id == uuid(1001)).unwrap();
        first_finished.white_rating = Some(2_444.0);
        first_finished.black_rating = Some(1_333.0);
        first_finished.updated_at = tournament.created_at + Duration::seconds(20);

        let plan = map_double_swiss(&DoubleSwissInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();
        let round = &plan.accepted_swiss_rounds[0];

        assert_eq!(
            round
                .accepted_ratings
                .iter()
                .find(|snapshot| snapshot.user_id == uuid(1))
                .unwrap()
                .rating,
            1_333
        );
        assert_eq!(
            round
                .accepted_ratings
                .iter()
                .find(|snapshot| snapshot.user_id == uuid(2))
                .unwrap()
                .rating,
            2_444
        );
    }

    #[test]
    fn rejects_a_round_without_complete_recoverable_ratings() {
        let (tournament, memberships, mut games) = valid_fixture();
        games
            .iter_mut()
            .find(|game| game.id == uuid(1000))
            .unwrap()
            .white_rating = None;

        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_missing_rating_snapshot"
        );
    }

    #[test]
    fn rejects_every_nonfinished_double_swiss_lifecycle() {
        for status in ["NotStarted", "InProgress"] {
            let (mut tournament, memberships, games) = valid_fixture();
            tournament.status = status.to_string();
            assert_eq!(
                diagnostic_code(map_double_swiss(&DoubleSwissInput {
                    tournament: &tournament,
                    memberships: &memberships,
                    games: &games,
                })),
                "legacy_double_swiss_nonfinished"
            );
        }
    }

    #[test]
    fn rejects_nonreciprocal_or_incomplete_historical_batches() {
        let (tournament, memberships, mut games) = valid_fixture();
        games[0].white_id = games[0].black_id;
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_self_pairing"
        );

        let (tournament, memberships, mut games) = valid_fixture();
        games.pop();
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_incomplete_round_batch"
        );
    }

    #[test]
    fn rejects_unknown_committee_shape_instead_of_guessing() {
        let (tournament, memberships, mut games) = valid_fixture();
        let committee = games
            .iter_mut()
            .find(|game| game.conclusion == "Committee")
            .unwrap();
        committee.turn = 1;
        committee.history = "wA1;".to_string();
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_unknown_committee_shape"
        );

        let (tournament, memberships, mut games) = valid_fixture();
        let committee = games.iter_mut().find(|game| game.id == uuid(1011)).unwrap();
        committee.tournament_game_result = String::from("1-0");
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_unknown_committee_shape"
        );
    }

    #[test]
    fn chooses_a_replay_leg_without_changing_either_game_color() {
        let (tournament, memberships, mut games) = valid_fixture();
        let earlier = games.iter_mut().find(|game| game.id == uuid(1001)).unwrap();
        earlier.created_at = tournament.created_at + Duration::milliseconds(1);
        earlier.updated_at = tournament.created_at + Duration::seconds(20);
        let later = games.iter_mut().find(|game| game.id == uuid(1000)).unwrap();
        later.created_at = tournament.created_at + Duration::milliseconds(2);
        later.updated_at = tournament.created_at + Duration::seconds(21);

        let plan = map_double_swiss(&DoubleSwissInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();
        let first = plan
            .slots
            .iter()
            .find(|slot| {
                matches!(
                    slot.status,
                    SlotPlanStatus::Sealed { game_id, .. } if game_id == uuid(1000)
                )
            })
            .unwrap();
        let second = plan
            .slots
            .iter()
            .find(|slot| {
                matches!(
                    slot.status,
                    SlotPlanStatus::Sealed { game_id, .. } if game_id == uuid(1001)
                )
            })
            .unwrap();
        assert!(matches!(
            first.key,
            SlotKey::Swiss {
                slot: SwissGameId {
                    leg: SwissLeg::First,
                    ..
                }
            }
        ));
        assert!(matches!(
            second.key,
            SlotKey::Swiss {
                slot: SwissGameId {
                    leg: SwissLeg::Second,
                    ..
                }
            }
        ));
        assert_eq!((first.white_id, first.black_id), (uuid(1), uuid(2)));
        assert_eq!((second.white_id, second.black_id), (uuid(2), uuid(1)));
    }

    #[test]
    fn rejects_unknown_scoring_and_tiebreakers() {
        let (mut tournament, memberships, games) = valid_fixture();
        tournament.scoring = "Match".to_string();
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_unknown_scoring"
        );

        let (mut tournament, memberships, games) = valid_fixture();
        tournament.tiebreaker.push(Some("Mystery".to_string()));
        assert_eq!(
            diagnostic_code(map_double_swiss(&DoubleSwissInput {
                tournament: &tournament,
                memberships: &memberships,
                games: &games,
            })),
            "legacy_double_swiss_unknown_tiebreaker"
        );
    }
}
