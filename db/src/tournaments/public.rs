use crate::{
    db_error::DbError,
    helpers::run_read_only_repeatable_read,
    models::{
        Rating,
        Tournament,
        TournamentFinalOutcome,
        TournamentFinishReason,
        TournamentInvitation,
        TournamentUser,
        User,
    },
    schema::{
        tournament_slots,
        tournament_swiss_rounds,
        tournaments_invitations,
        tournaments_organizer_invitations,
        tournaments_organizers,
        users,
    },
    tournaments::projection::{
        elimination_player_stats,
        elimination_projection,
        round_robin_player_stats,
        round_robin_projection,
        swiss_player_stats,
        swiss_projection,
    },
    DbConn,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde_json::{from_value, Value};
use shared_types::{
    tournament::{standings::Snapshot as StandingsSnapshot, Format, FormatConfig, SlotKey},
    tournament_view::{
        ArenaState,
        EliminationPlayerResultResponse,
        EliminationState,
        PlayerStatsResponse,
        RoundRobinState,
        SwissState,
        TournamentFormatResponse,
    },
    GameId,
    SwissProgress,
    SwissRoundSummary,
    TournamentStatus,
};
use std::{
    collections::{HashMap, HashSet},
    result::Result as StdResult,
};
use uuid::Uuid;

use super::{
    arena_projection_for_snapshot,
    elimination_finished_snapshot,
    evaluate_capabilities,
    load_for_read_with_memberships,
    project_elimination_facts,
    project_round_robin_facts,
    project_swiss_facts,
    round_robin_finished_snapshot,
    state::invalid_persisted,
    swiss_finished_snapshot,
    swiss_progress,
    FormatFacts,
    TournamentState,
};

type Result<T> = StdResult<T, DbError>;

enum FixedCardFormat {
    RoundRobin,
    Swiss { expected_rounds: u32 },
}

#[derive(Default)]
struct FixedCardSlotFacts {
    resolved: u32,
    total: u32,
    swiss_by_round: HashMap<u32, (u32, u32)>,
}

#[derive(Debug)]
pub struct TournamentProjection {
    pub standings: Option<StandingsSnapshot>,
    pub format: TournamentFormatResponse,
    pub player_stats: Vec<PlayerStatsResponse>,
}

#[derive(Clone, Debug)]
pub struct TournamentLoadRequest {
    pub memberships: bool,
    pub projection: Option<TournamentProjectionRequest>,
}

/// Selects entity payloads; standings and format metadata are always projected.
/// Only `All` is suitable for replacing the client's complete format state.
#[derive(Clone, Debug)]
pub enum TournamentProjectionRequest {
    All,
    Slots(Vec<Uuid>),
    Arena {
        games: Vec<GameId>,
        players: Vec<Uuid>,
    },
}

/// Page sections loaded together from one repeatable-read snapshot.
#[derive(Debug)]
pub struct TournamentSnapshot {
    pub tournament: Tournament,
    pub memberships: Option<TournamentMembershipSnapshot>,
    pub projection: Option<TournamentProjection>,
}

#[derive(Debug)]
pub struct TournamentMembershipSnapshot {
    pub memberships: Vec<TournamentUser>,
    pub organizer_ids: Vec<Uuid>,
    pub organizer_invitee_ids: Vec<Uuid>,
    pub invitations: Vec<TournamentInvitation>,
    pub users: Vec<User>,
    pub ratings: Vec<Rating>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedTournamentCardProgress {
    RoundRobin {
        resolved: u32,
        total: u32,
    },
    Swiss {
        completed_rounds: u32,
        total_rounds: u32,
        current_round: Option<SwissRoundSummary>,
    },
}

async fn load_in_snapshot(
    tournament: Tournament,
    request: TournamentLoadRequest,
    conn: &mut DbConn<'_>,
) -> Result<TournamentSnapshot> {
    let roster = if request.memberships
        || (request.projection.is_some() && tournament.configuration().format() != Format::Arena)
    {
        TournamentUser::find_by_tournament_id(tournament.id, conn).await?
    } else {
        Vec::new()
    };
    let projection = match request.projection {
        Some(selection) => {
            Some(project_in_snapshot(&tournament, roster.clone(), &selection, conn).await?)
        }
        None => None,
    };
    let memberships = if request.memberships {
        Some(load_memberships(&tournament, roster, conn).await?)
    } else {
        None
    };
    Ok(TournamentSnapshot {
        tournament,
        memberships,
        projection,
    })
}

async fn load_memberships(
    tournament: &Tournament,
    memberships: Vec<TournamentUser>,
    conn: &mut DbConn<'_>,
) -> Result<TournamentMembershipSnapshot> {
    let organizer_ids = tournaments_organizers::table
        .inner_join(users::table)
        .filter(users::deleted.eq(false))
        .filter(tournaments_organizers::tournament_id.eq(tournament.id))
        .select(tournaments_organizers::organizer_id)
        .load::<Uuid>(conn)
        .await?;
    let organizer_invitee_ids = tournaments_organizer_invitations::table
        .filter(tournaments_organizer_invitations::tournament_id.eq(tournament.id))
        .select(tournaments_organizer_invitations::invitee_id)
        .load::<Uuid>(conn)
        .await?;
    let invitations = tournaments_invitations::table
        .filter(tournaments_invitations::tournament_id.eq(tournament.id))
        .select(TournamentInvitation::as_select())
        .load(conn)
        .await?;
    let user_ids = memberships
        .iter()
        .map(|membership| membership.user_id)
        .chain(organizer_ids.iter().copied())
        .chain(organizer_invitee_ids.iter().copied())
        .chain(invitations.iter().map(|invitation| invitation.invitee_id))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let users = User::find_by_uuids(&user_ids, conn).await?;
    let ratings = Rating::for_uuids(&user_ids, conn).await?;
    Ok(TournamentMembershipSnapshot {
        memberships,
        organizer_ids,
        organizer_invitee_ids,
        invitations,
        users,
        ratings,
    })
}

async fn project_in_snapshot(
    tournament: &Tournament,
    memberships: Vec<TournamentUser>,
    selection: &TournamentProjectionRequest,
    conn: &mut DbConn<'_>,
) -> Result<TournamentProjection> {
    let configuration = tournament.configuration();
    let (selected_slots, selected_games, selected_players) = match selection {
        TournamentProjectionRequest::All => (None, None, None),
        TournamentProjectionRequest::Slots(slots) => (
            Some(slots.as_slice()),
            Some([].as_slice()),
            Some([].as_slice()),
        ),
        TournamentProjectionRequest::Arena { games, players } => (
            Some([].as_slice()),
            Some(games.as_slice()),
            Some(players.as_slice()),
        ),
    };
    let status = tournament.status();
    let frozen_outcome =
        if status == TournamentStatus::Finished && configuration.format() != Format::Arena {
            TournamentFinalOutcome::load(tournament.id, conn).await?
        } else {
            None
        };

    match (status, &configuration.format) {
        (TournamentStatus::NotStarted, FormatConfig::Arena(config)) => Ok(TournamentProjection {
            standings: None,
            format: TournamentFormatResponse::Arena(ArenaState {
                configuration: config.clone(),
                games: HashMap::new(),
                player_stats: HashMap::new(),
                featured_game_id: None,
            }),
            player_stats: Vec::new(),
        }),
        (TournamentStatus::NotStarted, FormatConfig::RoundRobin(config)) => {
            Ok(TournamentProjection {
                standings: None,
                format: TournamentFormatResponse::RoundRobin(RoundRobinState {
                    slots: HashMap::new(),
                    matches: Vec::new(),
                    configuration: config.clone(),
                    rounds: Vec::new(),
                    withdrawable_entrants: HashSet::new(),
                    closeout_eligible_slots: 0,
                }),
                player_stats: Vec::new(),
            })
        }
        (TournamentStatus::NotStarted, FormatConfig::Swiss(config)) => Ok(TournamentProjection {
            standings: None,
            format: TournamentFormatResponse::Swiss(SwissState {
                slots: HashMap::new(),
                configuration: config.clone(),
                withdrawable_entrants: HashSet::new(),
                closeout_eligible_slots: 0,
                rounds: Vec::new(),
                progress: SwissProgress::AwaitingResults,
            }),
            player_stats: Vec::new(),
        }),
        (TournamentStatus::NotStarted, FormatConfig::Elimination(config)) => {
            Ok(TournamentProjection {
                standings: None,
                format: TournamentFormatResponse::Elimination(EliminationState {
                    slots: HashMap::new(),
                    configuration: config.clone(),
                    withdrawable_entrants: HashSet::new(),
                    nodes: Vec::new(),
                    complete: false,
                    player_results: memberships
                        .iter()
                        .map(|membership| EliminationPlayerResultResponse {
                            player: membership.user_id,
                            in_contention: true,
                            placement: None,
                            exit_stage: None,
                        })
                        .collect(),
                }),
                player_stats: Vec::new(),
            })
        }
        (_, FormatConfig::Arena(_)) => {
            arena_projection_for_snapshot(tournament, selected_games, selected_players, conn).await
        }
        (_, FormatConfig::RoundRobin(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships, conn).await?;
            fixed_round_robin_snapshot(
                &state,
                frozen_outcome.map(|outcome| outcome.standings),
                selected_slots,
            )
        }
        (_, FormatConfig::Swiss(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships, conn).await?;
            fixed_swiss_snapshot(&state, frozen_outcome, selected_slots)
        }
        (_, FormatConfig::Elimination(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships, conn).await?;
            fixed_elimination_snapshot(
                &state,
                frozen_outcome.map(|outcome| outcome.standings),
                selected_slots,
            )
        }
    }
}

fn fixed_round_robin_snapshot(
    state: &TournamentState,
    frozen_standings: Option<StandingsSnapshot>,
    selected_slots: Option<&[Uuid]>,
) -> Result<TournamentProjection> {
    let projected = project_round_robin_facts(state)?;
    let capabilities = evaluate_capabilities(state, FormatFacts::RoundRobin(&projected))?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(round_robin_finished_snapshot(state, &projected)),
    };
    let format = round_robin_projection(state, &projected, &capabilities, selected_slots)?;
    let player_stats = round_robin_player_stats(state, &projected)?;
    Ok(TournamentProjection {
        standings,
        format,
        player_stats,
    })
}

fn fixed_swiss_snapshot(
    state: &TournamentState,
    frozen_outcome: Option<TournamentFinalOutcome>,
    selected_slots: Option<&[Uuid]>,
) -> Result<TournamentProjection> {
    let projected = project_swiss_facts(state)?;
    let (progress, frozen_standings) = if state.tournament.status() == TournamentStatus::Finished {
        let outcome = frozen_outcome
            .ok_or_else(|| invalid_persisted("Finished Swiss tournament has no final outcome"))?;
        let progress = match outcome.finish_reason {
            TournamentFinishReason::Completed => SwissProgress::Complete,
            TournamentFinishReason::PairingExhausted => SwissProgress::PairingExhausted,
        };
        (progress, Some(outcome.standings))
    } else {
        (swiss_progress(state, &projected), None)
    };
    let capabilities = evaluate_capabilities(state, FormatFacts::Swiss)?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(swiss_finished_snapshot(state, &projected)),
    };
    let format = swiss_projection(state, &projected, &capabilities, progress, selected_slots)?;
    let player_stats = swiss_player_stats(state, &projected)?;
    Ok(TournamentProjection {
        standings,
        format,
        player_stats,
    })
}

fn fixed_elimination_snapshot(
    state: &TournamentState,
    frozen_standings: Option<StandingsSnapshot>,
    selected_slots: Option<&[Uuid]>,
) -> Result<TournamentProjection> {
    let projected = project_elimination_facts(state)?;
    let capabilities = evaluate_capabilities(state, FormatFacts::Elimination(&projected))?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(elimination_finished_snapshot(state, &projected)),
    };
    let format = elimination_projection(state, &projected, &capabilities, selected_slots)?;
    let player_stats = elimination_player_stats(state, &projected)?;
    Ok(TournamentProjection {
        standings,
        format,
        player_stats,
    })
}

pub async fn load_by_id(id: Uuid, conn: &mut DbConn<'_>) -> Result<TournamentSnapshot> {
    load_sections(
        id,
        TournamentLoadRequest {
            memberships: true,
            projection: Some(TournamentProjectionRequest::All),
        },
        conn,
    )
    .await
}

/// Select page sections together; selected slots are intended for incremental updates.
/// A full format replacement must request `TournamentProjectionRequest::All`.
pub async fn load_sections(
    id: Uuid,
    request: TournamentLoadRequest,
    conn: &mut DbConn<'_>,
) -> Result<TournamentSnapshot> {
    run_read_only_repeatable_read(conn, move |tc| {
        let request = request.clone();
        Box::pin(
            async move { load_in_snapshot(Tournament::find(id, tc).await?, request, tc).await },
        )
    })
    .await
}

pub async fn fixed_card_progress_for_tournaments(
    tournaments: &[Tournament],
    conn: &mut DbConn<'_>,
) -> Result<HashMap<Uuid, FixedTournamentCardProgress>> {
    let mut formats = HashMap::new();
    for tournament in tournaments
        .iter()
        .filter(|tournament| tournament.status() == TournamentStatus::InProgress)
    {
        let configuration = tournament.configuration();
        match &configuration.format {
            FormatConfig::RoundRobin(_) => {
                formats.insert(tournament.id, FixedCardFormat::RoundRobin);
            }
            FormatConfig::Swiss(configuration) => {
                let expected_rounds = configuration
                    .rounds
                    .resolved_rounds()
                    .map(|rounds| rounds.get())
                    .ok_or_else(|| DbError::InvalidPersistedTournament {
                        reason: format!(
                            "in-progress Swiss tournament {} has unresolved rounds",
                            tournament.id
                        ),
                    })?;
                formats.insert(tournament.id, FixedCardFormat::Swiss { expected_rounds });
            }
            FormatConfig::Elimination(_) | FormatConfig::Arena(_) => {}
        }
    }
    let tournament_ids = formats.keys().copied().collect::<Vec<_>>();
    if tournament_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut slots = HashMap::<Uuid, FixedCardSlotFacts>::new();
    let slot_rows = tournament_slots::table
        .filter(tournament_slots::tournament_id.eq_any(&tournament_ids))
        .select((
            tournament_slots::tournament_id,
            tournament_slots::native_key,
            tournament_slots::resolution.is_not_null(),
        ))
        .load::<(Uuid, Value, bool)>(conn)
        .await?;
    for (tournament_id, key, resolved) in slot_rows {
        let facts = slots.entry(tournament_id).or_default();
        facts.resolved += u32::from(resolved);
        facts.total += 1;
        let key =
            from_value::<SlotKey>(key).map_err(|error| DbError::InvalidPersistedTournament {
                reason: format!("invalid slot key for tournament {tournament_id}: {error}"),
            })?;
        match key {
            SlotKey::Swiss { slot: game } => {
                let (round_resolved, round_total) =
                    facts.swiss_by_round.entry(game.round_index).or_default();
                *round_resolved += u32::from(resolved);
                *round_total += 1;
            }
            SlotKey::RoundRobin { .. } | SlotKey::Elimination { .. } => {}
        }
    }
    let mut accepted_swiss_rounds = HashMap::<Uuid, Vec<u32>>::new();
    let swiss_ids = formats
        .iter()
        .filter_map(|(id, format)| matches!(format, FixedCardFormat::Swiss { .. }).then_some(*id))
        .collect::<Vec<_>>();
    let round_rows = if swiss_ids.is_empty() {
        Vec::new()
    } else {
        tournament_swiss_rounds::table
            .filter(tournament_swiss_rounds::tournament_id.eq_any(&swiss_ids))
            .order((
                tournament_swiss_rounds::tournament_id,
                tournament_swiss_rounds::round_id,
            ))
            .select((
                tournament_swiss_rounds::tournament_id,
                tournament_swiss_rounds::round_id,
            ))
            .load::<(Uuid, i64)>(conn)
            .await?
    };
    for (tournament_id, round_id) in round_rows {
        let round_id =
            u32::try_from(round_id).map_err(|_| DbError::InvalidPersistedTournament {
                reason: format!("invalid Swiss round ID for tournament {tournament_id}"),
            })?;
        accepted_swiss_rounds
            .entry(tournament_id)
            .or_default()
            .push(round_id);
    }

    formats
        .into_iter()
        .map(|(tournament_id, format)| {
            let slot_facts = slots.remove(&tournament_id).unwrap_or_default();
            let progress = match format {
                FixedCardFormat::RoundRobin => FixedTournamentCardProgress::RoundRobin {
                    resolved: slot_facts.resolved,
                    total: slot_facts.total,
                },
                FixedCardFormat::Swiss { expected_rounds } => {
                    let rounds = accepted_swiss_rounds
                        .remove(&tournament_id)
                        .unwrap_or_default();
                    let projected_rounds = rounds
                        .iter()
                        .map(|round| {
                            let (resolved, total) = slot_facts
                                .swiss_by_round
                                .get(round)
                                .copied()
                                .unwrap_or_default();
                            (*round, resolved, total)
                        })
                        .collect::<Vec<_>>();
                    let completed_rounds = projected_rounds
                        .iter()
                        .filter(|(_, resolved, total)| resolved == total)
                        .count();
                    let completed_rounds = u32::try_from(completed_rounds)
                        .expect("admitted Swiss round count fits u32");
                    let current_round = projected_rounds
                        .last()
                        .filter(|(_, resolved, total)| resolved < total)
                        .map(|(round, resolved, total)| SwissRoundSummary {
                            round: round + 1,
                            resolved_games: *resolved,
                            total_games: *total,
                        });
                    FixedTournamentCardProgress::Swiss {
                        completed_rounds,
                        total_rounds: expected_rounds,
                        current_round,
                    }
                }
            };
            Ok((tournament_id, progress))
        })
        .collect()
}
