use crate::{
    db_error::DbError,
    models::{Game, TournamentSlot},
};
use hive_lib::GameStatus;
use shared_types::{
    tournament::{
        arena::{Config as ArenaConfig, PairingIntent},
        swiss::{PrimaryScore as DoubleSwissPrimaryScore, System as SwissSystem},
        FormatConfig,
        GameOutcome,
        Resolution,
        Score,
        SlotKey,
        SwissProgress,
    },
    tournament_view::{
        ArenaGameResponse,
        ArenaPlayerStatsResponse,
        CompactTournamentGameResponse,
        EliminationNodeResponse,
        EliminationNodeStateResponse,
        EliminationPlayerResultResponse,
        EliminationSeriesResponse,
        EliminationSeriesSetResponse,
        EliminationSeriesSlotResponse,
        PlayerStatsResponse,
        RoundRobinMatchResponse,
        RoundRobinRoundResponse,
        RoundRobinSlotResponse,
        SlotResponse,
        SwissByeResponse,
        SwissEncounterResponse,
        SwissMatchCompletionResponse,
        SwissRoundResponse,
        TournamentFormatResponse,
    },
    GameId,
    GameSpeed,
    GameStart,
};
use std::{collections::HashMap, str::FromStr};
use tournamint::{
    elimination::{EliminationNodeFactState, EliminationNodeId, EliminationStage},
    series::{self, SeriesGameFact, SeriesGameState},
    MatchScore,
    PlayerId,
};
use uuid::Uuid;

use super::{
    elimination_config,
    elimination_player_counts,
    elimination_series_facts_by_node,
    public_elimination_resolution,
    public_elimination_source,
    rating,
    ArenaFactsProjection,
    CapabilityProjection,
    EliminationFactsProjection,
    RoundRobinFactsProjection,
    SlotCapabilities,
    SwissFactsProjection,
    TournamentState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlayerRecord {
    player: Uuid,
    games_played: u32,
    matches_played: Option<u32>,
    wins: u32,
    draws: u32,
    losses: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RatingTotals {
    opponent_total: u64,
    performance_total: i64,
    games: u32,
}

impl RatingTotals {
    fn record(&mut self, opponent_rating: u32, result: MatchScore) -> Result<(), DbError> {
        let adjustment = match result {
            MatchScore::Win => 500,
            MatchScore::Draw => 0,
            MatchScore::Loss => -500,
        };
        let performance = i64::from(opponent_rating) + adjustment;
        self.opponent_total = self
            .opponent_total
            .checked_add(u64::from(opponent_rating))
            .ok_or_else(|| invalid_projection("opponent rating total overflowed"))?;
        self.performance_total = self
            .performance_total
            .checked_add(performance)
            .ok_or_else(|| invalid_projection("performance rating total overflowed"))?;
        self.games = self
            .games
            .checked_add(1)
            .ok_or_else(|| invalid_projection("rated game count overflowed"))?;
        Ok(())
    }

    fn merge(&mut self, other: Self) -> Result<(), DbError> {
        self.opponent_total = self
            .opponent_total
            .checked_add(other.opponent_total)
            .ok_or_else(|| invalid_projection("opponent rating total overflowed"))?;
        self.performance_total = self
            .performance_total
            .checked_add(other.performance_total)
            .ok_or_else(|| invalid_projection("performance rating total overflowed"))?;
        self.games = self
            .games
            .checked_add(other.games)
            .ok_or_else(|| invalid_projection("rated game count overflowed"))?;
        Ok(())
    }
}

struct FixedProjectionIndex<'a> {
    slots_by_key: HashMap<SlotKey, &'a TournamentSlot>,
    games_by_slot: HashMap<Uuid, &'a Game>,
}

impl<'a> FixedProjectionIndex<'a> {
    fn new(state: &'a TournamentState) -> Self {
        let slots_by_key = state.slots.iter().map(|slot| (slot.key, slot)).collect();
        let games_by_slot = state.games_by_slot();
        Self {
            slots_by_key,
            games_by_slot,
        }
    }

    fn slot(&self, key: SlotKey) -> &'a TournamentSlot {
        self.slots_by_key
            .get(&key)
            .copied()
            .expect("native projection references a persisted tournament Slot")
    }

    fn project_slot(
        &self,
        slot: &TournamentSlot,
        outcome: Option<GameOutcome>,
        awarded_game_points: Option<[Score; 2]>,
        capabilities: SlotCapabilities,
    ) -> Result<SlotResponse, DbError> {
        let game = self.games_by_slot.get(&slot.id).copied();
        let game = game.map(compact_game).transpose()?;
        Ok(SlotResponse {
            id: slot.id,
            key: slot.key,
            participants: [slot.white, slot.black],
            clock: slot.clock,
            resolution: slot.resolution,
            outcome,
            awarded_game_points,
            game,
            resolved_at: slot.resolved_at,
            scheduled_at: slot.scheduled_at,
            deadline_at: slot.deadline_at,
            waits_for: capabilities.waits_for,
            available_admin_actions: capabilities.admin_actions,
        })
    }
}

pub(crate) fn compact_game(game: &Game) -> Result<CompactTournamentGameResponse, DbError> {
    let game_status = GameStatus::from_str(&game.game_status).map_err(|error| {
        DbError::InvalidPersistedTournament {
            reason: format!("tournament game {} has invalid status: {error}", game.id),
        }
    })?;
    let game_start = GameStart::from_str(&game.game_start).map_err(|error| {
        DbError::InvalidPersistedTournament {
            reason: format!(
                "tournament game {} has invalid start state: {error}",
                game.id
            ),
        }
    })?;
    let speed =
        GameSpeed::from_str(&game.speed).map_err(|error| DbError::InvalidPersistedTournament {
            reason: format!("tournament game {} has invalid speed: {error}", game.id),
        })?;
    Ok(CompactTournamentGameResponse {
        game_id: GameId(game.nanoid.clone()),
        participants: [game.white_id, game.black_id],
        finished: game.finished,
        status: game_status,
        start: game_start,
        speed,
        finished_at: game.finished_at,
        ratings: [
            rating_snapshot(game.white_rating),
            rating_snapshot(game.black_rating),
        ],
        berserked: [game.white_berserked, game.black_berserked],
    })
}

fn record_rating_outcome(
    totals: &mut [RatingTotals; 2],
    ratings: [Option<u32>; 2],
    outcome: GameOutcome,
) -> Result<(), DbError> {
    let results = [outcome.white().score(), outcome.black().score()];
    for side in 0..2 {
        if let Some(opponent_rating) = ratings[1 - side] {
            totals[side].record(opponent_rating, results[side])?;
        }
    }
    Ok(())
}

fn fixed_rating_totals(
    state: &TournamentState,
    accepted_ratings: &HashMap<Uuid, [Option<u32>; 2]>,
) -> Result<HashMap<Uuid, RatingTotals>, DbError> {
    let games_by_slot = state.games_by_slot();
    let mut totals = state
        .memberships
        .iter()
        .map(|membership| (membership.user_id, RatingTotals::default()))
        .collect::<HashMap<_, _>>();
    for slot in &state.slots {
        let outcome = match slot.resolution {
            Some(Resolution::Result(outcome) | Resolution::Withdrawal(outcome)) => outcome,
            Some(Resolution::Clinched) | None => continue,
        };
        let game = games_by_slot.get(&slot.id).copied();
        if game.is_none() && !accepted_ratings.contains_key(&slot.id) {
            continue;
        }
        let accepted = accepted_ratings.get(&slot.id).copied().unwrap_or([None; 2]);
        let game = game.map_or([None; 2], |game| {
            [
                rating_snapshot(game.white_rating),
                rating_snapshot(game.black_rating),
            ]
        });
        let mut slot_totals = [RatingTotals::default(); 2];
        record_rating_outcome(
            &mut slot_totals,
            [accepted[0].or(game[0]), accepted[1].or(game[1])],
            outcome,
        )?;
        for (side, player) in [slot.white, slot.black].into_iter().enumerate() {
            let player_totals = totals
                .get_mut(&player)
                .expect("tournament Slot players belong to the persisted roster");
            player_totals.merge(slot_totals[side])?;
        }
    }
    Ok(totals)
}

fn fixed_player_stats(
    state: &TournamentState,
    records: impl IntoIterator<Item = PlayerRecord>,
    accepted_ratings: &HashMap<Uuid, [Option<u32>; 2]>,
) -> Result<Vec<PlayerStatsResponse>, DbError> {
    let totals = fixed_rating_totals(state, accepted_ratings)?;
    records
        .into_iter()
        .map(|record| {
            let rating = totals[&record.player];
            Ok(PlayerStatsResponse {
                player: record.player,
                performance_rating: rounded_signed_rating(rating.performance_total, rating.games)?,
                average_opponent_rating: rounded_unsigned_rating(
                    rating.opponent_total,
                    rating.games,
                )?,
                games_played: record.games_played,
                matches_played: record.matches_played,
                wins: record.wins,
                draws: record.draws,
                losses: record.losses,
            })
        })
        .collect()
}

pub(crate) fn round_robin_player_stats(
    state: &TournamentState,
    projected: &RoundRobinFactsProjection,
) -> Result<Vec<PlayerStatsResponse>, DbError> {
    let counts = projected
        .projection
        .players
        .iter()
        .map(|summary| PlayerRecord {
            player: player_uuid(state, summary.player),
            games_played: summary.games_played,
            matches_played: summary.matches_played,
            wins: summary.wins,
            draws: summary.draws,
            losses: summary.losses,
        })
        .collect::<Vec<_>>();
    fixed_player_stats(state, counts, &HashMap::new())
}

pub(crate) fn swiss_player_stats(
    state: &TournamentState,
    projected: &SwissFactsProjection,
) -> Result<Vec<PlayerStatsResponse>, DbError> {
    let double_swiss = matches!(
        &state.configuration.format,
        FormatConfig::Swiss(configuration)
            if matches!(configuration.system, SwissSystem::DoubleSwiss(_))
    );
    let counts = projected
        .projection
        .players
        .iter()
        .map(|summary| PlayerRecord {
            player: player_uuid(state, summary.player),
            games_played: summary.games_played,
            matches_played: double_swiss.then_some(summary.matches_played),
            wins: summary.game_wins,
            draws: summary.game_draws,
            losses: summary.game_losses,
        })
        .collect::<Vec<_>>();
    let accepted_ratings = swiss_slot_rating_snapshots(state);
    fixed_player_stats(state, counts, &accepted_ratings)
}

pub(crate) fn elimination_player_stats(
    state: &TournamentState,
    projected: &EliminationFactsProjection,
) -> Result<Vec<PlayerStatsResponse>, DbError> {
    let counts = elimination_player_counts(state, &projected.projection);
    let counts = counts
        .into_iter()
        .zip(&state.memberships)
        .map(|(counts, membership)| PlayerRecord {
            player: membership.user_id,
            games_played: counts.games_played,
            matches_played: Some(counts.matches_played),
            wins: counts.wins,
            draws: counts.draws,
            losses: counts.losses,
        });
    fixed_player_stats(state, counts, &HashMap::new())
}

fn swiss_slot_rating_snapshots(state: &TournamentState) -> HashMap<Uuid, [Option<u32>; 2]> {
    let players = state
        .memberships
        .iter()
        .enumerate()
        .map(|(index, membership)| (membership.user_id, PlayerId::new(index)))
        .collect::<HashMap<_, _>>();
    let mut rounds = HashMap::with_capacity(state.swiss_rounds.len());
    for row in &state.swiss_rounds {
        rounds.insert(row.native_round_id(), row.accepted_rating_snapshots());
    }
    state
        .slots
        .iter()
        .filter_map(|slot| {
            let SlotKey::Swiss { slot: game } = slot.key else {
                return None;
            };
            Some((slot, game))
        })
        .map(|(slot, game)| {
            let ratings = rounds[&game.round_index];
            let player_rating = |player: Uuid| Some(ratings[&players[&player]]);
            (
                slot.id,
                [player_rating(slot.white), player_rating(slot.black)],
            )
        })
        .collect()
}

pub(crate) fn round_robin_projection(
    state: &TournamentState,
    projected: &RoundRobinFactsProjection,
    capabilities: &CapabilityProjection,
) -> Result<TournamentFormatResponse, DbError> {
    let index = FixedProjectionIndex::new(state);
    let rounds = projected
        .projection
        .rounds
        .iter()
        .map(|round| {
            let resting = round.resting.map(|player| player_uuid(state, player));
            let slots = round
                .games
                .iter()
                .map(|game| {
                    let key = SlotKey::RoundRobin {
                        slot: game.scheduled_game.id,
                    };
                    let slot = index.slot(key);
                    let completed = game.completed;
                    let capabilities = slot_capabilities(capabilities, slot.id);
                    index
                        .project_slot(
                            slot,
                            completed.map(|completed| completed.outcome),
                            completed.map(|completed| completed.awarded_points),
                            capabilities,
                        )
                        .map(|slot| RoundRobinSlotResponse {
                            board_index: game.scheduled_game.board_index,
                            slot,
                        })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            Ok(RoundRobinRoundResponse {
                round_index: round.round_index,
                pass_index: round.pass_index,
                resting,
                slots,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    let matches = projected
        .projection
        .matches
        .iter()
        .map(|encounter| RoundRobinMatchResponse {
            participants: encounter
                .participants
                .map(|player| player_uuid(state, player)),
            completion: encounter.completed,
        })
        .collect();
    Ok(TournamentFormatResponse::RoundRobin {
        configuration: match &state.configuration.format {
            FormatConfig::RoundRobin(config) => config.clone(),
            _ => unreachable!(),
        },
        rounds,
        matches,
        withdrawable_entrants: capabilities.withdrawable_entrants.clone(),
        closeout_eligible_slots: capabilities.closeout_eligible_slots.unwrap_or(0),
    })
}

pub(crate) fn swiss_projection(
    state: &TournamentState,
    projected: &SwissFactsProjection,
    capabilities: &CapabilityProjection,
    progress: SwissProgress,
) -> Result<TournamentFormatResponse, DbError> {
    let index = FixedProjectionIndex::new(state);
    let primary_is_match_points = matches!(
        &state.configuration.format,
        FormatConfig::Swiss(configuration)
            if matches!(
                configuration.system,
                SwissSystem::DoubleSwiss(system)
                    if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
            )
    );
    let mut primary_scores = vec![Score::new(0); state.memberships.len()];
    let rounds = projected
        .projection
        .rounds
        .iter()
        .zip(&state.swiss_rounds)
        .map(|(round, row)| {
            let accepted_ratings = row.accepted_rating_snapshots();
            let encounters = round
                .encounters
                .iter()
                .map(|encounter| {
                    let pairing_players = [encounter.pairing.white(), encounter.pairing.black()];
                    let participants = [
                        player_uuid(state, pairing_players[0]),
                        player_uuid(state, pairing_players[1]),
                    ];
                    let pre_round_primary_scores = [
                        swiss_primary_score(&primary_scores, pairing_players[0]),
                        swiss_primary_score(&primary_scores, pairing_players[1]),
                    ];
                    let slots = encounter
                        .games
                        .iter()
                        .map(|game| {
                            let key = SlotKey::Swiss { slot: game.id };
                            let slot = index.slot(key);
                            let capabilities = slot_capabilities(capabilities, slot.id);
                            index.project_slot(
                                slot,
                                game.completed.map(|completed| completed.outcome),
                                game.completed.map(|completed| completed.awarded_points),
                                capabilities,
                            )
                        })
                        .collect::<Result<Vec<_>, DbError>>()?;
                    let rating_snapshots = [
                        Some(accepted_ratings[&pairing_players[0]]),
                        Some(accepted_ratings[&pairing_players[1]]),
                    ];
                    Ok(SwissEncounterResponse {
                        pairing_index: encounter.pairing_index,
                        participants,
                        pre_round_primary_scores,
                        rating_snapshots,
                        slots,
                        completion: encounter.completion.map(|completion| {
                            SwissMatchCompletionResponse {
                                dispositions: completion.dispositions,
                                aggregate: completion.aggregate,
                                game_points: completion.game_points,
                                match_points: completion.match_points,
                            }
                        }),
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            let byes = round
                .byes
                .iter()
                .map(|bye_| SwissByeResponse {
                    player: player_uuid(state, bye_.player),
                    pre_round_primary_score: swiss_primary_score(&primary_scores, bye_.player),
                    rating_snapshot: Some(accepted_ratings[&bye_.player]),
                    game_points: bye_.game_points,
                    match_points: bye_.match_points,
                })
                .collect::<Vec<_>>();
            for encounter in &round.encounters {
                let Some(completion) = encounter.completion else {
                    continue;
                };
                let awards = if primary_is_match_points {
                    completion.match_points
                } else {
                    completion.game_points
                };
                add_swiss_primary_score(&mut primary_scores, encounter.pairing.white(), awards[0])?;
                add_swiss_primary_score(&mut primary_scores, encounter.pairing.black(), awards[1])?;
            }
            for bye_ in &round.byes {
                add_swiss_primary_score(
                    &mut primary_scores,
                    bye_.player,
                    if primary_is_match_points {
                        bye_.match_points
                    } else {
                        bye_.game_points
                    },
                )?;
            }
            Ok(SwissRoundResponse {
                round_index: round.round_index,
                encounters,
                byes,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    Ok(TournamentFormatResponse::Swiss {
        configuration: match &state.configuration.format {
            FormatConfig::Swiss(config) => config.clone(),
            _ => unreachable!(),
        },
        rounds,
        progress,
        withdrawable_entrants: capabilities.withdrawable_entrants.clone(),
        closeout_eligible_slots: capabilities.closeout_eligible_slots.unwrap_or(0),
    })
}

fn swiss_primary_score(scores: &[Score], player: PlayerId) -> Score {
    scores[player.index()]
}

fn add_swiss_primary_score(
    scores: &mut [Score],
    player: PlayerId,
    award: Score,
) -> Result<(), DbError> {
    let score = &mut scores[player.index()];
    let value = score
        .value()
        .checked_add(award.value())
        .ok_or_else(|| invalid_projection("Swiss primary score overflowed"))?;
    *score = Score::new(value);
    Ok(())
}

pub(crate) fn elimination_projection(
    state: &TournamentState,
    projected: &EliminationFactsProjection,
    capabilities: &CapabilityProjection,
) -> Result<TournamentFormatResponse, DbError> {
    let index = FixedProjectionIndex::new(state);
    let series_facts = elimination_series_facts_by_node(state);
    let nodes = projected
        .projection
        .nodes
        .iter()
        .map(|node| {
            let descriptor = node.descriptor;
            let node_series_facts = series_facts
                .get(&descriptor.id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let (entrants, state_projection, series_entrants) = match node.state {
                EliminationNodeFactState::Planned => (
                    [None; 2],
                    EliminationNodeStateResponse::Planned {
                        conditional: descriptor.stage == EliminationStage::Reset,
                    },
                    None,
                ),
                EliminationNodeFactState::Active { entrants } => (
                    [
                        Some(player_uuid(state, entrants[0])),
                        Some(player_uuid(state, entrants[1])),
                    ],
                    EliminationNodeStateResponse::Active,
                    Some(entrants),
                ),
                EliminationNodeFactState::Resolved {
                    entrants,
                    resolution,
                } => {
                    let public_entrants = [
                        entrants[0].map(|player| player_uuid(state, player)),
                        entrants[1].map(|player| player_uuid(state, player)),
                    ];
                    let series_entrants = match entrants {
                        [Some(first), Some(second)] if !node_series_facts.is_empty() => {
                            Some([first, second])
                        }
                        _ => None,
                    };
                    (
                        public_entrants,
                        EliminationNodeStateResponse::Resolved(public_elimination_resolution(
                            state, resolution,
                        )),
                        series_entrants,
                    )
                }
                EliminationNodeFactState::Skipped => {
                    ([None; 2], EliminationNodeStateResponse::Skipped, None)
                }
            };
            let possible_entrants = [
                node.possible_entrants[0]
                    .iter()
                    .copied()
                    .map(|player| player_uuid(state, player))
                    .collect::<Vec<_>>(),
                node.possible_entrants[1]
                    .iter()
                    .copied()
                    .map(|player| player_uuid(state, player))
                    .collect::<Vec<_>>(),
            ];
            let series = series_entrants
                .map(|entrants| {
                    elimination_series_projection(
                        state,
                        &index,
                        capabilities,
                        descriptor.id,
                        descriptor.stage,
                        entrants,
                        node_series_facts,
                    )
                })
                .transpose()?;
            Ok(EliminationNodeResponse {
                node_id: descriptor.id,
                wave_index: descriptor.scheduled_wave_index,
                stage: descriptor.stage,
                stage_ordinal: descriptor.stage_ordinal,
                sources: [
                    public_elimination_source(descriptor.sources[0]),
                    public_elimination_source(descriptor.sources[1]),
                ],
                state: state_projection,
                entrants,
                possible_entrants,
                series,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    let mut ranked_results = projected.projection.players.iter().collect::<Vec<_>>();
    ranked_results.sort_by_key(|result| result.player.index());
    let player_results = ranked_results
        .into_iter()
        .map(|result| EliminationPlayerResultResponse {
            player: player_uuid(state, result.player),
            in_contention: result.in_contention,
            placement: result.placement,
            exit_stage: result.exit_stage,
        })
        .collect::<Vec<_>>();
    Ok(TournamentFormatResponse::Elimination {
        configuration: elimination_config(&state.configuration)?.clone(),
        withdrawable_entrants: capabilities.withdrawable_entrants.clone(),
        nodes,
        complete: projected.projection.complete,
        player_results,
        reset_required: projected.projection.reset_required,
    })
}

pub(crate) fn arena_projection(
    projected: &ArenaFactsProjection,
    featured_game_id: Option<Uuid>,
    configuration: &ArenaConfig,
) -> Result<TournamentFormatResponse, DbError> {
    let mut ordered_games = projected.games.iter().collect::<Vec<_>>();
    ordered_games.sort_unstable_by_key(|game| game.ordinal);
    let results_by_ordinal = projected
        .results
        .iter()
        .map(|result| (result.ordinal, result))
        .collect::<HashMap<_, _>>();
    let games = ordered_games
        .iter()
        .map(|game| {
            let ordinal = game.ordinal;
            let result = results_by_ordinal.get(&ordinal).copied();
            compact_game(&game.game).map(|game| ArenaGameResponse {
                ordinal,
                game,
                outcome: result.map(|result| result.outcome),
                awarded_points: result.map(|result| result.points),
                doubled: result.map(|result| result.doubled),
                no_start_absent: result.and_then(|result| result.no_start_absent),
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    let player_stats = arena_player_stats(projected)?;
    let featured_game_id = featured_game_id.and_then(|featured_id| {
        ordered_games
            .iter()
            .find(|game| game.game.id == featured_id)
            .map(|game| GameId(game.game.nanoid.clone()))
    });
    Ok(TournamentFormatResponse::Arena {
        configuration: configuration.clone(),
        games,
        player_stats,
        featured_game_id,
    })
}

fn elimination_series_projection(
    state: &TournamentState,
    index: &FixedProjectionIndex<'_>,
    capabilities: &CapabilityProjection,
    node: EliminationNodeId,
    stage: EliminationStage,
    entrants: [PlayerId; 2],
    facts: &[SeriesGameFact],
) -> Result<EliminationSeriesResponse, DbError> {
    let config = elimination_config(&state.configuration)?;
    let native_plan = config.effective_plan(stage).to_native().map_err(|error| {
        DbError::InvalidPersistedTournament {
            reason: format!(
                "elimination node {} has invalid series plan: {error}",
                node.value()
            ),
        }
    })?;
    let projected = series::project(&native_plan, entrants, facts).map_err(|error| {
        DbError::InvalidPersistedTournament {
            reason: format!(
                "elimination node {} has invalid series facts: {error}",
                node.value()
            ),
        }
    })?;
    let sets = projected
        .sets
        .iter()
        .map(|set| {
            let slots = set
                .games
                .iter()
                .map(|game| {
                    let slot = index.slot(SlotKey::Elimination {
                        node,
                        slot: game.id,
                    });
                    let outcome = match game.state {
                        SeriesGameState::Completed(outcome) => Some(outcome),
                        SeriesGameState::Planned
                        | SeriesGameState::Released
                        | SeriesGameState::Skipped => None,
                    };
                    let capabilities = slot_capabilities(capabilities, slot.id);
                    index
                        .project_slot(slot, outcome, None, capabilities)
                        .map(|slot| EliminationSeriesSlotResponse { slot })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            Ok(EliminationSeriesSetResponse { slots })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    Ok(EliminationSeriesResponse {
        score: [projected.score.first, projected.score.second],
        sets,
    })
}

fn slot_capabilities(capabilities: &CapabilityProjection, slot_id: Uuid) -> SlotCapabilities {
    capabilities.slots[&slot_id].clone()
}

fn player_uuid(state: &TournamentState, player: PlayerId) -> Uuid {
    state.memberships[player.index()].user_id
}

fn invalid_projection(reason: impl Into<String>) -> DbError {
    DbError::InvalidPersistedTournament {
        reason: reason.into(),
    }
}

fn rating_snapshot(rating: Option<f64>) -> Option<u32> {
    rating.and_then(rating::snapshot)
}

fn rounded_signed_rating(total: i64, games: u32) -> Result<Option<i32>, DbError> {
    rounded_signed_value(total, games)
        .map(|rounded| {
            i32::try_from(rounded)
                .map_err(|_| invalid_projection("performance rating exceeds the supported domain"))
        })
        .transpose()
}

pub(crate) fn rounded_signed_value(total: i64, games: u32) -> Option<i64> {
    if games == 0 {
        return None;
    }
    let denominator = i64::from(games);
    let quotient = total / denominator;
    let remainder = total % denominator;
    let adjustment = if remainder.unsigned_abs() >= denominator.unsigned_abs().div_ceil(2) {
        total.signum()
    } else {
        0
    };
    quotient.checked_add(adjustment)
}

fn rounded_unsigned_rating(total: u64, games: u32) -> Result<Option<u32>, DbError> {
    if games == 0 {
        return Ok(None);
    }
    let denominator = u64::from(games);
    let quotient = total / denominator;
    let remainder = total % denominator;
    let rounded = quotient + u64::from(remainder >= denominator.div_ceil(2));
    u32::try_from(rounded)
        .map(Some)
        .map_err(|_| invalid_projection("average opponent rating exceeds the supported domain"))
}

pub(crate) fn arena_player_stats(
    projected: &ArenaFactsProjection,
) -> Result<Vec<ArenaPlayerStatsResponse>, DbError> {
    let mut rating_totals = vec![RatingTotals::default(); projected.players.len()];
    for result in &projected.results {
        let mut game = [RatingTotals::default(); 2];
        record_rating_outcome(&mut game, result.ratings, result.outcome)?;
        rating_totals[result.pairing.white().index()].merge(game[0])?;
        rating_totals[result.pairing.black().index()].merge(game[1])?;
    }
    projected
        .players
        .iter()
        .zip(rating_totals)
        .map(|(player, rating)| {
            let metrics = &player.metrics;
            let membership = &player.membership;
            let performance = metrics.performance;
            Ok(ArenaPlayerStatsResponse {
                player: membership.user_id,
                points: metrics.points,
                performance_rating: performance
                    .map(|performance| {
                        rounded_signed_rating(performance.numerator(), performance.denominator())
                    })
                    .transpose()?
                    .flatten(),
                average_opponent_rating: rounded_unsigned_rating(
                    rating.opponent_total,
                    rating.games,
                )?,
                performance_games: performance.map_or(0, |performance| performance.denominator()),
                arena_rating: metrics.arena_rating,
                games_scored: metrics.games_scored,
                games_played: metrics.games_played,
                no_starts: metrics.no_starts,
                wins: metrics.wins,
                draws: metrics.draws,
                losses: metrics.losses,
                current_streak: metrics.current_streak,
                on_fire: metrics.on_fire,
                best_streak: metrics.best_streak,
                berserks: metrics.berserks,
                paused: matches!(membership.pairing_intent()?, Some(PairingIntent::Paused)),
            })
        })
        .collect()
}
