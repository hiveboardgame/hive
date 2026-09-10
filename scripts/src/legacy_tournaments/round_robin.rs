use super::{
    legacy_standings::{LegacyStandings, LegacyTiebreaker},
    model::{
        AuditDiagnostic,
        FinalOutcomePlan,
        LegacyGameRow,
        LegacyMembershipRow,
        LegacyRankGroup,
        LegacyTournamentRow,
        LegacyTournamentStatus,
        Participant,
        Roster,
        SlotPlan,
        SlotPlanStatus,
        SupportedLegacyMapping,
        TournamentPlan,
    },
};
use chrono::Duration;
use db_lib::tournaments::build_round_robin_config;
use hive_lib::{Color, GameResult, GameStatus};
use shared_types::{
    tournament::{
        round_robin::{
            Config as RoundRobinConfig,
            Criterion as RoundRobinCriterion,
            DirectEncounterOptions,
            SonnebornBergerOptions,
        },
        standings::{Group, Placement, Row, Snapshot, Value as StandingValue},
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        BotAdmission,
        Clock,
        Config,
        DirectEncounterForfeitPolicy,
        FormatConfig,
        GameOutcome,
        PlayedGameOutcome,
        PointSystem,
        ReleasePolicy,
        RepeatedEncounterPolicy,
        RoundRobinGameId,
        SlotKey,
    },
    Conclusion,
    TimeMode,
    TournamentGameResult,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    num::NonZeroU32,
    str::FromStr,
};
use tournamint::{
    round_robin::{
        project,
        schedule,
        RoundRobinGameFact,
        RoundRobinGameId as NativeGameId,
        RoundRobinProjection,
        RoundRobinSchedule,
    },
    MatchPointSystem,
    PlayerId,
};
use uuid::Uuid;

pub struct RoundRobinInput<'a> {
    pub tournament: &'a LegacyTournamentRow,
    pub memberships: &'a [LegacyMembershipRow],
    pub games: &'a [LegacyGameRow],
}

pub struct RoundRobinMapping {
    pub plan: TournamentPlan,
    pub diagnostics: Vec<AuditDiagnostic>,
}

#[derive(Clone)]
struct LogicalGame {
    native_game_id: RoundRobinGameId,
    key: SlotKey,
    white_id: Uuid,
    black_id: Uuid,
}

pub fn map_round_robin(
    input: &RoundRobinInput<'_>,
) -> Result<RoundRobinMapping, Vec<AuditDiagnostic>> {
    map_round_robin_inner(input).map_err(|message| {
        vec![AuditDiagnostic::hard_failure(
            Some(input.tournament.id),
            "round_robin_mapping",
            message,
        )]
    })
}

fn map_round_robin_inner(input: &RoundRobinInput<'_>) -> Result<RoundRobinMapping, String> {
    let tournament = input.tournament;
    let repeats = repeats(&tournament.mode)?;
    if tournament.scoring != "Game" {
        return Err(format!(
            "legacy Round Robin scoring {:?} is unsupported",
            tournament.scoring
        ));
    }
    if tournament.rounds != 1 {
        return Err(format!(
            "legacy Round Robin rounds must be 1, found {}",
            tournament.rounds
        ));
    }
    let status = LegacyTournamentStatus::from_str(&tournament.status)
        .map_err(|error| format!("invalid legacy tournament status: {error}"))?;
    let deadline_warning = deadline_metadata(tournament, status)?;
    let clock = tournament_clock(tournament)?;
    let configuration = Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::RoundRobin(RoundRobinConfig {
            primary_score: Default::default(),
            match_point_system: MatchPointSystem::STANDARD,
            repeats,
            release_policy: ReleasePolicy::FullyUnlocked,
            clock,
            game_point_system: PointSystem::STANDARD,
            standings: standings_plan(&tournament.tiebreaker)?,
        }),
    };

    match status {
        LegacyTournamentStatus::NotStarted => {
            if tournament.started_at.is_some() {
                return Err(String::from(
                    "a NotStarted legacy Round Robin has a started timestamp",
                ));
            }
            if !input.games.is_empty() {
                return Err(String::from(
                    "a NotStarted legacy Round Robin already owns games",
                ));
            }
            configuration
                .validate_for_creation()
                .map_err(|error| format!("invalid mapped Round Robin configuration: {error}"))?;
            return Ok(RoundRobinMapping {
                plan: TournamentPlan {
                    tournament: tournament.clone(),
                    mapping: SupportedLegacyMapping::RoundRobin,
                    configuration,
                    roster: None,
                    slots: Vec::new(),
                    accepted_swiss_rounds: Vec::new(),
                    expected_rank_groups: Vec::new(),
                    final_outcome: None,
                },
                diagnostics: deadline_warning.into_iter().collect(),
            });
        }
        LegacyTournamentStatus::InProgress | LegacyTournamentStatus::Finished => {
            if tournament.started_at.is_none() {
                return Err(String::from(
                    "a started legacy Round Robin has no started timestamp",
                ));
            }
        }
    }

    let roster = roster(input.memberships)?;
    let players = (0..roster.participants.len())
        .map(PlayerId::new)
        .collect::<Vec<_>>();
    let FormatConfig::RoundRobin(round_robin) = &configuration.format else {
        unreachable!("the mapped configuration was constructed as Round Robin")
    };
    let native_config = build_round_robin_config(round_robin);
    let schedule = schedule(&native_config, &players)
        .map_err(|error| format!("could not schedule mapped Round Robin: {error}"))?;
    let logical_games = logical_games(&schedule, &roster)?;
    let bindings = bind_games(&logical_games, input.games)?;

    let mut native_results = HashMap::new();
    let mut legacy_standings = LegacyStandings::default();
    let legacy_criteria = parse_legacy_tiebreakers(&tournament.tiebreaker)?;
    let mut slots = Vec::with_capacity(logical_games.len());
    for logical in &logical_games {
        let game = bindings
            .get(&logical.native_game_id)
            .copied()
            .ok_or_else(|| String::from("an expected Round Robin game was not bound"))?;
        validate_game_clock(game, tournament)?;
        let legacy_result = TournamentGameResult::from_str(&game.tournament_game_result)
            .map_err(|error| format!("game {} has an invalid result: {error}", game.id))?;
        legacy_standings
            .add_result(game.white_id, game.black_id, legacy_result.clone())
            .map_err(|error| {
                format!("game {} is invalid for legacy standings: {error}", game.id)
            })?;
        let normalized = normalize_round_robin_game(game)?;
        let status = match normalized {
            Some(outcome) => {
                native_results.insert(NativeGameId::new(logical.native_game_id.value()), outcome);
                SlotPlanStatus::Sealed {
                    game_id: game.id,
                    outcome,
                }
            }
            None => SlotPlanStatus::Released { game_id: game.id },
        };
        slots.push(SlotPlan {
            key: logical.key,
            white_id: logical.white_id,
            black_id: logical.black_id,
            clock,
            status,
        });
    }

    let game_facts = native_results
        .into_iter()
        .map(|(id, outcome)| RoundRobinGameFact { id, outcome })
        .collect::<Vec<_>>();
    let projection = project(&native_config, &players, &game_facts)
        .map_err(|error| format!("mapped Round Robin results are invalid: {error}"))?;
    let actual_groups = native_rank_groups(&projection, &roster)?;
    let expected_groups = legacy_standings
        .rank_groups(&legacy_criteria)
        .map_err(|error| format!("could not reconstruct legacy standings: {error}"))?;

    let mut diagnostics = deadline_warning.into_iter().collect::<Vec<_>>();
    match status {
        LegacyTournamentStatus::Finished => {
            if !projection.is_complete() {
                return Err(String::from(
                    "a Finished legacy Round Robin still has unresolved games",
                ));
            }
            if actual_groups != expected_groups {
                return Err(format!(
                    "finished Round Robin rank groups differ: legacy {expected_groups:?}, mapped {actual_groups:?}"
                ));
            }
        }
        LegacyTournamentStatus::InProgress => {
            if projection.is_complete() {
                diagnostics.push(AuditDiagnostic::warning(
                    Some(tournament.id),
                    "round_robin_engine_complete_while_in_progress",
                    "legacy status is InProgress although mapped Round Robin replay is complete",
                ));
            }
            if actual_groups != expected_groups {
                diagnostics.push(AuditDiagnostic::warning(
                    Some(tournament.id),
                    "round_robin_live_order_changed",
                    "mapped ongoing standings groups differ from the legacy ordering",
                ));
            }
        }
        LegacyTournamentStatus::NotStarted => unreachable!(),
    }

    let final_outcome = matches!(status, LegacyTournamentStatus::Finished)
        .then(|| round_robin_final_outcome(&projection, &roster))
        .transpose()?;

    Ok(RoundRobinMapping {
        plan: TournamentPlan {
            tournament: tournament.clone(),
            mapping: SupportedLegacyMapping::RoundRobin,
            configuration,
            roster: Some(roster),
            slots,
            accepted_swiss_rounds: Vec::new(),
            expected_rank_groups: expected_groups,
            final_outcome,
        },
        diagnostics,
    })
}

fn normalize_round_robin_game(game: &LegacyGameRow) -> Result<Option<GameOutcome>, String> {
    let status = GameStatus::from_str(&game.game_status)
        .map_err(|error| format!("invalid game status: {error}"))?;
    let result = TournamentGameResult::from_str(&game.tournament_game_result)
        .map_err(|error| format!("invalid tournament result: {error}"))?;
    let conclusion = Conclusion::from_str(&game.conclusion)
        .map_err(|error| format!("invalid game conclusion: {error}"))?;
    if !game.finished {
        if !matches!(status, GameStatus::NotStarted | GameStatus::InProgress)
            || result != TournamentGameResult::Unknown
            || conclusion != Conclusion::Unknown
        {
            return Err(String::from(
                "an unfinished Round Robin game has terminal state",
            ));
        }
        return Ok(None);
    }

    match status {
        GameStatus::Finished(game_result) => {
            let outcome = match (game_result, result) {
                (GameResult::Winner(Color::White), TournamentGameResult::Winner(Color::White)) => {
                    PlayedGameOutcome::WhiteWin
                }
                (GameResult::Winner(Color::Black), TournamentGameResult::Winner(Color::Black)) => {
                    PlayedGameOutcome::BlackWin
                }
                (GameResult::Draw, TournamentGameResult::Draw) => PlayedGameOutcome::Draw,
                _ => {
                    return Err(String::from(
                        "game result disagrees with its tournament result",
                    ));
                }
            };
            let decisive = matches!(
                outcome,
                PlayedGameOutcome::WhiteWin | PlayedGameOutcome::BlackWin
            );
            if !matches!(
                conclusion,
                Conclusion::Resigned | Conclusion::Withdrawal if decisive
            ) && !(conclusion == Conclusion::Draw && outcome == PlayedGameOutcome::Draw)
                && !(conclusion == Conclusion::Timeout && decisive)
                && conclusion != Conclusion::Board
                && !(conclusion == Conclusion::Repetition && outcome == PlayedGameOutcome::Draw)
            {
                return Err(String::from("played game has an incompatible conclusion"));
            }
            Ok(Some(GameOutcome::Played(outcome)))
        }
        GameStatus::Adjudicated if conclusion == Conclusion::Committee => {
            ensure_untouched_adjudication(game, "Committee")?;
            let outcome = match result {
                TournamentGameResult::Winner(Color::White) => AdjudicatedGameOutcome::new(
                    AdjudicatedSideResult::ForfeitWin,
                    AdjudicatedSideResult::ForfeitLoss,
                ),
                TournamentGameResult::Winner(Color::Black) => AdjudicatedGameOutcome::new(
                    AdjudicatedSideResult::ForfeitLoss,
                    AdjudicatedSideResult::ForfeitWin,
                ),
                TournamentGameResult::Draw => AdjudicatedGameOutcome::new(
                    AdjudicatedSideResult::Draw,
                    AdjudicatedSideResult::Draw,
                ),
                TournamentGameResult::Unknown | TournamentGameResult::DoubleForfeit => {
                    return Err(String::from(
                        "a Round Robin Committee result must be a win or draw",
                    ));
                }
            }
            .expect("recognized Round Robin committee outcomes are valid");
            Ok(Some(GameOutcome::Adjudicated(outcome)))
        }
        GameStatus::Adjudicated if conclusion == Conclusion::Forfeit => {
            ensure_untouched_adjudication(game, "Forfeit")?;
            if result != TournamentGameResult::DoubleForfeit {
                return Err(String::from(
                    "a Forfeit adjudication must be the recognized double-forfeit result",
                ));
            }
            Ok(Some(GameOutcome::Adjudicated(
                AdjudicatedGameOutcome::new(
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit,
                )
                .expect("valid double forfeit"),
            )))
        }
        GameStatus::Adjudicated => Err(String::from(
            "adjudicated Round Robin game has an unsupported conclusion or result shape",
        )),
        GameStatus::NotStarted | GameStatus::InProgress => Err(String::from(
            "a finished Round Robin game still has a nonterminal status",
        )),
    }
}

fn ensure_untouched_adjudication(game: &LegacyGameRow, label: &str) -> Result<(), String> {
    if game.turn == 0
        && game.history.is_empty()
        && game.game_control_history.is_empty()
        && game.hashes.is_empty()
        && game.move_times.is_empty()
        && game.game_start == "Ready"
    {
        Ok(())
    } else {
        Err(format!(
            "{label} game is not an untouched Ready turn-0 adjudication"
        ))
    }
}

fn repeats(mode: &str) -> Result<NonZeroU32, String> {
    let repeats = match mode {
        "DoubleRoundRobin" => 2,
        "QuadrupleRoundRobin" => 4,
        "SextupleRoundRobin" => 6,
        other => return Err(format!("unsupported legacy Round Robin mode {other:?}")),
    };
    NonZeroU32::new(repeats).ok_or_else(|| String::from("Round Robin repeats cannot be zero"))
}

fn deadline_metadata(
    tournament: &LegacyTournamentRow,
    status: LegacyTournamentStatus,
) -> Result<Option<AuditDiagnostic>, String> {
    let accepted_dropped_metadata = match (tournament.round_duration, tournament.ends_at) {
        (None, None) => return Ok(None),
        (Some(_), _) if tournament.time_mode != "Real Time" => false,
        (Some(days), None) => days > 0 && status == LegacyTournamentStatus::NotStarted,
        (Some(days), Some(ends_at)) => {
            let latest_end = tournament.started_at.and_then(|started_at| {
                Duration::try_days(i64::from(days))
                    .and_then(|duration| started_at.checked_add_signed(duration))
            });
            days > 0
                && status != LegacyTournamentStatus::NotStarted
                && tournament.started_at.is_some_and(|started_at| {
                    latest_end
                        .is_some_and(|latest_end| ends_at > started_at && ends_at <= latest_end)
                })
        }
        (None, Some(_)) => false,
    };
    if !accepted_dropped_metadata {
        return Err(format!(
            "legacy Round Robin deadline metadata is incoherent for status {}: round_duration={:?}, started_at={:?}, ends_at={:?}",
            tournament.status,
            tournament.round_duration,
            tournament.started_at,
            tournament.ends_at
        ));
    }
    Ok(Some(AuditDiagnostic::warning(
        Some(tournament.id),
        "round_robin_legacy_deadline_dropped",
        "legacy fixed-duration metadata is preserved in the frozen source but has no current Round Robin runtime equivalent",
    )))
}

fn tournament_clock(tournament: &LegacyTournamentRow) -> Result<Clock, String> {
    let mode = TimeMode::from_str(&tournament.time_mode)
        .map_err(|error| format!("invalid legacy time mode: {error}"))?;
    Clock::from_time_parts(mode, tournament.time_base, tournament.time_increment)
        .map_err(|error| format!("invalid legacy tournament clock: {error}"))?
        .ok_or_else(|| String::from("legacy tournaments cannot map to an untimed clock"))
}

fn standings_plan(legacy: &[Option<String>]) -> Result<Vec<RoundRobinCriterion>, String> {
    let mut mapped = vec![RoundRobinCriterion::PrimaryScore];
    for criterion in parse_legacy_tiebreakers(legacy)? {
        let criterion = match criterion {
            LegacyTiebreaker::RawPoints => RoundRobinCriterion::PrimaryScore,
            LegacyTiebreaker::HeadToHead => {
                RoundRobinCriterion::DirectEncounter(DirectEncounterOptions {
                    forfeits: DirectEncounterForfeitPolicy::IncludeAsScored,
                    repeated_encounters: RepeatedEncounterPolicy::Sum,
                })
            }
            LegacyTiebreaker::WinsAsBlack => RoundRobinCriterion::GamesWonWithBlack,
            LegacyTiebreaker::SonnebornBerger => {
                RoundRobinCriterion::SonnebornBerger(SonnebornBergerOptions { cut_lowest: 0 })
            }
        };
        if !mapped.contains(&criterion) {
            mapped.push(criterion);
        }
    }
    Ok(mapped)
}

fn parse_legacy_tiebreakers(legacy: &[Option<String>]) -> Result<Vec<LegacyTiebreaker>, String> {
    legacy
        .iter()
        .map(|criterion| {
            let criterion = criterion
                .as_deref()
                .ok_or_else(|| String::from("legacy tiebreakers contain a NULL entry"))?;
            LegacyTiebreaker::from_str(criterion)
        })
        .collect()
}

fn roster(memberships: &[LegacyMembershipRow]) -> Result<Roster, String> {
    let mut user_ids = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect::<Vec<_>>();
    user_ids.sort_unstable();
    if user_ids.len() < 2 {
        return Err(String::from(
            "a started Round Robin requires at least two members",
        ));
    }
    if user_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(String::from("legacy Round Robin membership is duplicated"));
    }
    let participants = user_ids
        .into_iter()
        .enumerate()
        .map(|(index, user_id)| {
            Ok(Participant {
                user_id,
                pairing_number: u32::try_from(index + 1)
                    .map_err(|_| String::from("Round Robin roster is too large"))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Roster { participants })
}

fn logical_games(
    schedule: &RoundRobinSchedule,
    roster: &Roster,
) -> Result<Vec<LogicalGame>, String> {
    schedule
        .rounds
        .iter()
        .flat_map(|round| round.games.iter())
        .map(|game| {
            let native_game_id = game.id;
            let white_id = roster
                .participants
                .get(game.white.index())
                .ok_or_else(|| String::from("native White is absent from the roster"))?
                .user_id;
            let black_id = roster
                .participants
                .get(game.black.index())
                .ok_or_else(|| String::from("native Black is absent from the roster"))?
                .user_id;
            Ok(LogicalGame {
                native_game_id,
                key: SlotKey::RoundRobin {
                    slot: native_game_id,
                },
                white_id,
                black_id,
            })
        })
        .collect()
}

fn bind_games<'a>(
    logical_games: &[LogicalGame],
    legacy_games: &'a [LegacyGameRow],
) -> Result<HashMap<RoundRobinGameId, &'a LegacyGameRow>, String> {
    let mut logical_by_pair = BTreeMap::<(Uuid, Uuid), Vec<&LogicalGame>>::new();
    for game in logical_games {
        logical_by_pair
            .entry((game.white_id, game.black_id))
            .or_default()
            .push(game);
    }
    for games in logical_by_pair.values_mut() {
        games.sort_by_key(|game| game.native_game_id);
    }
    let mut source_games_by_pair = BTreeMap::<(Uuid, Uuid), Vec<&LegacyGameRow>>::new();
    for game in legacy_games {
        source_games_by_pair
            .entry((game.white_id, game.black_id))
            .or_default()
            .push(game);
    }
    for games in source_games_by_pair.values_mut() {
        games.sort_by_key(|game| (game.created_at, game.id));
    }
    let pairs = logical_by_pair
        .keys()
        .chain(source_games_by_pair.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut bindings = HashMap::new();
    for pair in pairs {
        let logical = logical_by_pair.get(&pair).map(Vec::as_slice).unwrap_or(&[]);
        let source_games = source_games_by_pair
            .get(&pair)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if logical.len() != source_games.len() {
            return Err(format!(
                "directed pairing {}-{} has {} native games and {} legacy games",
                pair.0,
                pair.1,
                logical.len(),
                source_games.len()
            ));
        }
        for (logical, source_game) in logical.iter().zip(source_games) {
            bindings.insert(logical.native_game_id, *source_game);
        }
    }
    if bindings.len() != logical_games.len() || bindings.len() != legacy_games.len() {
        return Err(String::from(
            "legacy Round Robin source-game binding is not bijective",
        ));
    }
    Ok(bindings)
}

fn validate_game_clock(
    game: &LegacyGameRow,
    tournament: &LegacyTournamentRow,
) -> Result<(), String> {
    if game.tournament_id != Some(tournament.id) {
        return Err(format!(
            "game {} has foreign tournament ownership {:?}",
            game.id, game.tournament_id
        ));
    }
    if game.time_mode != tournament.time_mode
        || game.time_base != tournament.time_base
        || game.time_increment != tournament.time_increment
    {
        return Err(format!(
            "game {} clock differs from its legacy tournament",
            game.id
        ));
    }
    Ok(())
}

fn native_rank_groups(
    projection: &RoundRobinProjection,
    roster: &Roster,
) -> Result<Vec<LegacyRankGroup>, String> {
    projection
        .standings
        .groups
        .iter()
        .map(|group| {
            let mut user_ids = group
                .players
                .iter()
                .map(|standing| {
                    roster
                        .participants
                        .get(standing.player.index())
                        .map(|participant| participant.user_id)
                        .ok_or_else(|| {
                            String::from("mapped standings contain a player outside the roster")
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            user_ids.sort_unstable();
            Ok(LegacyRankGroup {
                competition_rank: group.rank,
                user_ids,
            })
        })
        .collect()
}

fn round_robin_final_outcome(
    projection: &RoundRobinProjection,
    roster: &Roster,
) -> Result<FinalOutcomePlan, String> {
    let mut groups = Vec::with_capacity(projection.standings.groups.len());
    for group in &projection.standings.groups {
        let mut rows = Vec::with_capacity(group.players.len());
        for standing in &group.players {
            let participant = roster
                .participants
                .get(standing.player.index())
                .ok_or_else(|| {
                    String::from("final Round Robin standings reference an unknown player")
                })?;
            let summary = projection
                .players
                .iter()
                .find(|summary| summary.player == standing.player)
                .ok_or_else(|| String::from("final Round Robin standings omit a player summary"))?;
            rows.push(Row {
                user_id: participant.user_id,
                primary_score: StandingValue::Score(summary.primary_score),
                games_played: summary.games_played,
                matches_played: summary.matches_played,
                wins: summary.wins,
                draws: summary.draws,
                losses: summary.losses,
                counts: vec![
                    summary.rests,
                    summary.match_wins,
                    summary.match_draws,
                    summary.match_losses,
                ],
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn tournament(mode: &str, status: &str) -> LegacyTournamentRow {
        LegacyTournamentRow {
            id: Uuid::from_u128(1),
            nanoid: String::from("legacy"),
            name: String::from("Legacy"),
            description: String::new(),
            scoring: String::from("Game"),
            tiebreaker: vec![
                Some(String::from("RawPoints")),
                Some(String::from("HeadToHead")),
                Some(String::from("WinsAsBlack")),
                Some(String::from("SonnebornBerger")),
            ],
            seats: 8,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: mode.to_string(),
            time_mode: String::from("Real Time"),
            time_base: Some(180),
            time_increment: Some(2),
            band_upper: None,
            band_lower: None,
            start_mode: String::from("Manual"),
            starts_at: None,
            ends_at: None,
            started_at: (status != "NotStarted").then(|| Utc.timestamp_opt(2, 0).unwrap()),
            round_duration: None,
            status: status.to_string(),
            created_at: Utc.timestamp_opt(1, 0).unwrap(),
            updated_at: Utc.timestamp_opt(3, 0).unwrap(),
            series: None,
        }
    }

    fn memberships(tournament_id: Uuid) -> [LegacyMembershipRow; 2] {
        [
            LegacyMembershipRow {
                tournament_id,
                user_id: Uuid::from_u128(10),
            },
            LegacyMembershipRow {
                tournament_id,
                user_id: Uuid::from_u128(20),
            },
        ]
    }

    fn game(
        tournament: &LegacyTournamentRow,
        id: u128,
        white_id: Uuid,
        black_id: Uuid,
        created_offset: i64,
        result: Option<&str>,
    ) -> LegacyGameRow {
        let created_at = tournament.created_at + Duration::seconds(created_offset);
        let finished = result.is_some();
        LegacyGameRow {
            id: Uuid::from_u128(id),
            nanoid: format!("game-{id}"),
            current_player_id: white_id,
            black_id,
            finished,
            game_status: result
                .map(|result| format!("Finished({result})"))
                .unwrap_or_else(|| String::from("InProgress")),
            history: if finished { "wA1;" } else { "" }.to_string(),
            game_control_history: String::new(),
            turn: usize::from(finished) as i32,
            white_id,
            created_at,
            updated_at: created_at + Duration::seconds(1),
            time_mode: tournament.time_mode.clone(),
            time_base: tournament.time_base,
            time_increment: tournament.time_increment,
            white_rating: Some(1_500.0),
            black_rating: Some(1_500.0),
            hashes: Vec::new(),
            conclusion: if finished {
                String::from("Board")
            } else {
                String::from("Unknown")
            },
            tournament_id: Some(tournament.id),
            tournament_game_result: result.unwrap_or("Unknown").to_string(),
            game_start: String::from("Ready"),
            move_times: Vec::new(),
        }
    }

    #[test]
    fn maps_exact_legacy_repeat_modes_without_materializing_not_started_state() {
        for (mode, expected) in [
            ("DoubleRoundRobin", 2),
            ("QuadrupleRoundRobin", 4),
            ("SextupleRoundRobin", 6),
        ] {
            let mut tournament = tournament(mode, "NotStarted");
            let starts_at = tournament.updated_at + Duration::hours(1);
            tournament.starts_at = Some(starts_at);
            let mapped = map_round_robin(&RoundRobinInput {
                tournament: &tournament,
                memberships: &[],
                games: &[],
            })
            .unwrap();
            let FormatConfig::RoundRobin(config) = mapped.plan.configuration.format else {
                panic!("expected Round Robin configuration");
            };
            assert_eq!(config.repeats.get(), expected);
            assert_eq!(config.release_policy, ReleasePolicy::FullyUnlocked);
            assert_eq!(mapped.plan.tournament.starts_at, Some(starts_at));
            assert!(mapped.plan.roster.is_none());
            assert!(mapped.plan.slots.is_empty());
        }
    }

    #[test]
    fn mapped_standings_prepend_points_and_deduplicate_like_the_legacy_app() {
        let mut tournament = tournament("DoubleRoundRobin", "NotStarted");
        tournament.tiebreaker = vec![
            Some(String::from("HeadToHead")),
            Some(String::from("RawPoints")),
            Some(String::from("HeadToHead")),
            Some(String::from("SonnebornBerger")),
        ];
        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &[],
            games: &[],
        })
        .unwrap();
        let FormatConfig::RoundRobin(config) = mapped.plan.configuration.format else {
            panic!("expected Round Robin configuration");
        };
        assert_eq!(
            config.standings,
            vec![
                RoundRobinCriterion::PrimaryScore,
                RoundRobinCriterion::DirectEncounter(DirectEncounterOptions {
                    forfeits: DirectEncounterForfeitPolicy::IncludeAsScored,
                    repeated_encounters: RepeatedEncounterPolicy::Sum,
                },),
                RoundRobinCriterion::SonnebornBerger(SonnebornBergerOptions { cut_lowest: 0 },),
            ]
        );
    }

    #[test]
    fn coherent_legacy_deadline_metadata_is_preserved_and_warned() {
        let mut not_started = tournament("SextupleRoundRobin", "NotStarted");
        not_started.round_duration = Some(90);
        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &not_started,
            memberships: &[],
            games: &[],
        })
        .unwrap();
        assert_eq!(mapped.plan.tournament.round_duration, Some(90));
        assert_eq!(mapped.diagnostics.len(), 1);
        assert_eq!(
            mapped.diagnostics[0].code,
            "round_robin_legacy_deadline_dropped"
        );
    }

    #[test]
    fn inconsistent_legacy_deadline_metadata_fails_closed() {
        let mut tournament = tournament("DoubleRoundRobin", "Finished");
        tournament.round_duration = Some(1);
        tournament.ends_at = tournament.started_at;

        assert!(deadline_metadata(&tournament, LegacyTournamentStatus::Finished).is_err());
    }

    #[test]
    fn unknown_mode_and_criterion_fail_closed() {
        let mut unknown_mode = tournament("SingleRoundRobin", "NotStarted");
        assert!(map_round_robin(&RoundRobinInput {
            tournament: &unknown_mode,
            memberships: &[],
            games: &[],
        })
        .is_err());

        unknown_mode.mode = String::from("DoubleRoundRobin");
        unknown_mode.tiebreaker.push(Some(String::from("Buchholz")));
        assert!(map_round_robin(&RoundRobinInput {
            tournament: &unknown_mode,
            memberships: &[],
            games: &[],
        })
        .is_err());
    }

    #[test]
    fn started_mapping_preserves_directed_open_and_terminal_games() {
        let tournament = tournament("DoubleRoundRobin", "InProgress");
        let memberships = memberships(tournament.id);
        let first = memberships[0].user_id;
        let second = memberships[1].user_id;
        let games = vec![
            game(&tournament, 101, second, first, 2, None),
            game(&tournament, 100, first, second, 1, Some("1-0")),
        ];
        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();

        assert_eq!(mapped.plan.slots.len(), 2);
        assert_eq!(
            mapped
                .plan
                .slots
                .iter()
                .filter(|slot| matches!(slot.status, SlotPlanStatus::Sealed { .. }))
                .count(),
            1
        );
        assert!(mapped.plan.accepted_swiss_rounds.is_empty());
        assert!(mapped.diagnostics.is_empty());
    }

    #[test]
    fn committee_results_translate_as_round_robin_administrative_outcomes() {
        let tournament = tournament("DoubleRoundRobin", "InProgress");
        let memberships = memberships(tournament.id);
        let first = memberships[0].user_id;
        let second = memberships[1].user_id;
        let mut committee = game(&tournament, 100, first, second, 1, Some("1-0"));
        committee.game_status = String::from("Adjudicated");
        committee.conclusion = String::from("Committee");
        committee.history.clear();
        committee.turn = 0;
        let games = vec![
            committee.clone(),
            game(&tournament, 101, second, first, 2, Some("0-1")),
        ];

        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();
        assert!(mapped.plan.slots.iter().any(|slot| matches!(
            slot.status,
            SlotPlanStatus::Sealed {
                game_id,
                outcome: GameOutcome::Adjudicated(_),
            } if game_id == committee.id
        )));

        committee.turn = 1;
        assert!(map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &[
                committee,
                game(&tournament, 101, second, first, 2, Some("0-1")),
            ],
        })
        .is_err());
    }

    #[test]
    fn complete_in_progress_mapping_warns_without_rewriting_lifecycle() {
        let tournament = tournament("DoubleRoundRobin", "InProgress");
        let memberships = memberships(tournament.id);
        let first = memberships[0].user_id;
        let second = memberships[1].user_id;
        let games = vec![
            game(&tournament, 100, first, second, 1, Some("1-0")),
            game(&tournament, 101, second, first, 2, Some("0-1")),
        ];
        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();

        assert_eq!(mapped.plan.tournament.status, "InProgress");
        assert!(mapped.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "round_robin_engine_complete_while_in_progress"
        }));
        assert!(mapped.plan.final_outcome.is_none());
    }

    #[test]
    fn finished_mapping_requires_and_preserves_exact_competition_rank_groups() {
        let tournament = tournament("DoubleRoundRobin", "Finished");
        let memberships = memberships(tournament.id);
        let first = memberships[0].user_id;
        let second = memberships[1].user_id;
        let games = vec![
            game(&tournament, 100, first, second, 1, Some("1-0")),
            game(&tournament, 101, second, first, 2, Some("0-1")),
        ];
        let mapped = map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .unwrap();

        assert!(mapped.diagnostics.is_empty());
        assert_eq!(mapped.plan.expected_rank_groups.len(), 2);
        assert_eq!(mapped.plan.expected_rank_groups[0].competition_rank, 1);
        assert_eq!(mapped.plan.expected_rank_groups[0].user_ids, vec![first]);
        assert_eq!(mapped.plan.expected_rank_groups[1].competition_rank, 2);
        assert_eq!(mapped.plan.expected_rank_groups[1].user_ids, vec![second]);
        let frozen = &mapped.plan.final_outcome.as_ref().unwrap().standings;
        let frozen_rank_groups = frozen
            .groups
            .iter()
            .map(|group| LegacyRankGroup {
                competition_rank: group.placement.rank(),
                user_ids: group.rows.iter().map(|row| row.user_id).collect(),
            })
            .collect::<Vec<_>>();
        assert_eq!(frozen_rank_groups, mapped.plan.expected_rank_groups);
    }

    #[test]
    fn missing_directed_game_fails_bijection() {
        let tournament = tournament("DoubleRoundRobin", "InProgress");
        let memberships = memberships(tournament.id);
        let games = [game(
            &tournament,
            100,
            memberships[0].user_id,
            memberships[1].user_id,
            1,
            None,
        )];
        assert!(map_round_robin(&RoundRobinInput {
            tournament: &tournament,
            memberships: &memberships,
            games: &games,
        })
        .is_err());
    }
}
