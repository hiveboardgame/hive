use crate::{
    db_error::DbError,
    helpers::run_read_only_repeatable_read,
    models::{
        Rating,
        Tournament,
        TournamentFinalOutcome,
        TournamentInvitation,
        TournamentSlot,
        TournamentSwissRound,
        TournamentUser,
        User,
    },
    schema::{
        tournaments_invitations,
        tournaments_organizer_invitations,
        tournaments_organizers,
        users,
    },
    tournaments::projection::{
        arena_player_stats,
        arena_projection,
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
use shared_types::{
    tournament::{standings::Snapshot as StandingsSnapshot, Format, FormatConfig, SlotKey},
    tournament_view::{
        ArenaPlayerStatsResponse,
        EliminationPlayerResultResponse,
        PlayerStatsResponse,
        TournamentFormatResponse,
    },
    SwissProgress,
    SwissRoundSummary,
    TournamentId,
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

struct ProjectedSnapshot {
    standings: Option<StandingsSnapshot>,
    format: TournamentFormatResponse,
    player_stats: Vec<PlayerStatsResponse>,
}

/// Everything required by the tournament endpoint from one repeatable-read
/// snapshot. Format facts are already projected and UUID-normalized here.
#[derive(Debug)]
pub struct TournamentSnapshot {
    pub tournament: Tournament,
    pub memberships: Vec<TournamentUser>,
    pub organizer_ids: Vec<Uuid>,
    pub organizer_invitee_ids: Vec<Uuid>,
    pub invitations: Vec<TournamentInvitation>,
    pub users: Vec<User>,
    pub ratings: Vec<Rating>,
    pub standings: Option<StandingsSnapshot>,
    pub format: TournamentFormatResponse,
    pub player_stats: Vec<PlayerStatsResponse>,
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
    conn: &mut DbConn<'_>,
) -> Result<TournamentSnapshot> {
    let configuration = tournament.configuration();
    let memberships = TournamentUser::find_by_tournament_id(tournament.id, conn).await?;
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
    let status = tournament.status();
    let frozen_standings =
        if status == TournamentStatus::Finished && configuration.format() != Format::Arena {
            TournamentFinalOutcome::load(tournament.id, conn)
                .await?
                .map(|outcome| outcome.standings)
        } else {
            None
        };

    let projected = match (status, &configuration.format) {
        (TournamentStatus::NotStarted, FormatConfig::Arena(config)) => ProjectedSnapshot {
            standings: None,
            format: TournamentFormatResponse::Arena {
                configuration: config.clone(),
                games: Vec::new(),
                player_stats: Vec::new(),
                featured_game_id: None,
            },
            player_stats: Vec::new(),
        },
        (TournamentStatus::NotStarted, FormatConfig::RoundRobin(config)) => ProjectedSnapshot {
            standings: None,
            format: TournamentFormatResponse::RoundRobin {
                matches: Vec::new(),
                configuration: config.clone(),
                rounds: Vec::new(),
                withdrawable_entrants: HashSet::new(),
                closeout_eligible_slots: 0,
            },
            player_stats: Vec::new(),
        },
        (TournamentStatus::NotStarted, FormatConfig::Swiss(config)) => ProjectedSnapshot {
            standings: None,
            format: TournamentFormatResponse::Swiss {
                configuration: config.clone(),
                withdrawable_entrants: HashSet::new(),
                closeout_eligible_slots: 0,
                rounds: Vec::new(),
                progress: SwissProgress::AwaitingResults,
            },
            player_stats: Vec::new(),
        },
        (TournamentStatus::NotStarted, FormatConfig::Elimination(config)) => ProjectedSnapshot {
            standings: None,
            format: TournamentFormatResponse::Elimination {
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
            },
            player_stats: Vec::new(),
        },
        (_, FormatConfig::Arena(config)) => {
            let projected = arena_projection_for_snapshot(&tournament, conn).await?;
            let standings = Some(projected.standings.clone());
            ProjectedSnapshot {
                standings,
                format: arena_projection(&projected, tournament.featured_game_id, config)?,
                player_stats: Vec::new(),
            }
        }
        (_, FormatConfig::RoundRobin(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships.clone(), conn)
                    .await?;
            fixed_round_robin_snapshot(&state, frozen_standings)?
        }
        (_, FormatConfig::Swiss(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships.clone(), conn)
                    .await?;
            fixed_swiss_snapshot(&state, frozen_standings)?
        }
        (_, FormatConfig::Elimination(_)) => {
            let state =
                load_for_read_with_memberships(tournament.clone(), memberships.clone(), conn)
                    .await?;
            fixed_elimination_snapshot(&state, frozen_standings)?
        }
    };

    Ok(TournamentSnapshot {
        tournament,
        memberships,
        organizer_ids,
        organizer_invitee_ids,
        invitations,
        users,
        ratings,
        standings: projected.standings,
        format: projected.format,
        player_stats: projected.player_stats,
    })
}

fn fixed_round_robin_snapshot(
    state: &TournamentState,
    frozen_standings: Option<StandingsSnapshot>,
) -> Result<ProjectedSnapshot> {
    let projected = project_round_robin_facts(state)?;
    let capabilities = evaluate_capabilities(state, FormatFacts::RoundRobin(&projected))?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(round_robin_finished_snapshot(state, &projected)),
    };
    let format = round_robin_projection(state, &projected, &capabilities)?;
    let player_stats = round_robin_player_stats(state, &projected)?;
    Ok(ProjectedSnapshot {
        standings,
        format,
        player_stats,
    })
}

fn fixed_swiss_snapshot(
    state: &TournamentState,
    frozen_standings: Option<StandingsSnapshot>,
) -> Result<ProjectedSnapshot> {
    let projected = project_swiss_facts(state)?;
    let progress = swiss_progress(state, &projected)?;
    let capabilities = evaluate_capabilities(state, FormatFacts::Swiss)?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(swiss_finished_snapshot(state, &projected)),
    };
    let format = swiss_projection(state, &projected, &capabilities, progress)?;
    let player_stats = swiss_player_stats(state, &projected)?;
    Ok(ProjectedSnapshot {
        standings,
        format,
        player_stats,
    })
}

fn fixed_elimination_snapshot(
    state: &TournamentState,
    frozen_standings: Option<StandingsSnapshot>,
) -> Result<ProjectedSnapshot> {
    let projected = project_elimination_facts(state)?;
    let capabilities = evaluate_capabilities(state, FormatFacts::Elimination(&projected))?;
    let standings = match frozen_standings {
        Some(standings) => Some(standings),
        None => Some(elimination_finished_snapshot(state, &projected)),
    };
    let format = elimination_projection(state, &projected, &capabilities)?;
    let player_stats = elimination_player_stats(state, &projected)?;
    Ok(ProjectedSnapshot {
        standings,
        format,
        player_stats,
    })
}

pub async fn load_by_id(id: Uuid, conn: &mut DbConn<'_>) -> Result<TournamentSnapshot> {
    run_read_only_repeatable_read(conn, move |tc| {
        Box::pin(async move { load_in_snapshot(Tournament::find(id, tc).await?, tc).await })
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
    for slot in TournamentSlot::find_by_tournament_ids(&tournament_ids, conn).await? {
        let tournament_id = slot.tournament_id;
        let resolved = slot.resolution.is_some();
        let facts = slots.entry(tournament_id).or_default();
        facts.resolved += u32::from(resolved);
        facts.total += 1;
        match slot.key {
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
    for row in TournamentSwissRound::find_by_tournament_ids(&tournament_ids, conn).await? {
        let tournament_id = row.tournament_id;
        accepted_swiss_rounds
            .entry(tournament_id)
            .or_default()
            .push(row.native_round_id());
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
                            resolved_encounters: *resolved,
                            total_encounters: *total,
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

/// Reuses the authoritative Arena calculation without assembling unrelated public sections.
pub async fn load_arena_player_stats(
    id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<(TournamentId, Vec<ArenaPlayerStatsResponse>)> {
    run_read_only_repeatable_read(conn, move |tc| {
        Box::pin(async move {
            let tournament = Tournament::find(id, tc).await?;
            let facts = arena_projection_for_snapshot(&tournament, tc).await?;
            Ok((TournamentId(tournament.nanoid), arena_player_stats(&facts)?))
        })
    })
    .await
}
