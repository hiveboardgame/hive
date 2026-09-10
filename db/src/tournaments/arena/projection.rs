use crate::{
    db_error::DbError,
    models::{Game, Tournament, TournamentFinalOutcome, TournamentUser},
    tournaments::{projection::rounded_signed_value, ArenaResultSnapshot},
    DbConn,
};
use shared_types::{
    tournament::{
        standings::{Group, Placement, Row, Snapshot, Value},
        Score,
    },
    TournamentStatus,
};
use std::collections::HashMap;
use tournamint::{
    arena::{
        self,
        ArenaConfig as TournamintArenaConfig,
        ArenaProjection,
        ArenaStandingMetrics,
        ArenaTerminalResult,
    },
    AdjudicatedGameOutcome,
    AdjudicatedSideResult,
    GameOutcome,
};
use uuid::Uuid;

use super::{
    super::state::invalid_persisted,
    state::{
        arena_configuration,
        awarded_facts,
        load_arena_state_for_snapshot,
        projection_players,
        terminal_facts,
        terminal_facts_before,
        ArenaDbState,
        ArenaTerminalFact,
    },
};

pub(crate) struct ArenaFactsProjection {
    pub(crate) standings: Snapshot,
    pub(crate) results: Vec<ArenaResultSnapshot>,
    pub(crate) games: Vec<ArenaProjectionGame>,
    pub(crate) players: Vec<ArenaProjectionPlayer>,
}

pub(crate) struct ArenaProjectionGame {
    pub(crate) game: Game,
    pub(crate) ordinal: i64,
}

pub(crate) struct ArenaProjectionPlayer {
    pub(crate) membership: TournamentUser,
    pub(crate) metrics: ArenaStandingMetrics,
}

pub(crate) async fn arena_projection_for_snapshot(
    tournament: &Tournament,
    conn: &mut DbConn<'_>,
) -> Result<ArenaFactsProjection, DbError> {
    let configuration = arena_configuration(tournament)?.clone();
    let state = load_arena_state_for_snapshot(tournament.clone(), configuration, conn).await?;
    let mut players = projection_players(&state);
    let finished = tournament.status() == TournamentStatus::Finished;
    let terminals = if finished {
        if state.games.iter().any(|game| game.terminal.is_none()) {
            return Err(invalid_persisted(
                "Finished Arena includes a nonterminal Game",
            ));
        }
        terminal_facts(&state.games)?
    } else {
        terminal_facts_before(&state.games, state.boundaries.ends_at)?
    };
    let awarded = awarded_facts(&terminals)?;
    let config = TournamintArenaConfig::default();
    let (standings, metrics) = if finished {
        let frozen = TournamentFinalOutcome::load(tournament.id, conn)
            .await?
            .ok_or_else(|| invalid_persisted("Finished Arena is missing its frozen outcome"))?;
        let ratings = frozen
            .arena_ratings
            .as_ref()
            .ok_or_else(|| invalid_persisted("Finished Arena is missing its frozen ratings"))?;
        let mut ratings_by_player = ratings
            .iter()
            .map(|rating| (rating.user_id, rating.rating))
            .collect::<HashMap<_, _>>();
        if ratings_by_player.len() != ratings.len() {
            return Err(invalid_persisted(
                "Finished Arena has duplicate frozen ratings",
            ));
        }
        for (player, membership) in players.iter_mut().zip(&state.memberships) {
            player.arena_rating = Some(ratings_by_player.remove(&membership.user_id).ok_or_else(
                || invalid_persisted("Finished Arena is missing a participant rating"),
            )?);
        }
        if !ratings_by_player.is_empty() {
            return Err(invalid_persisted(
                "Finished Arena has an unknown participant rating",
            ));
        }
        let metrics = arena::player_metrics(&config, &players, &awarded)
            .map_err(|error| invalid_persisted(&format!("Arena facts are invalid: {error}")))?;
        (frozen.standings, metrics)
    } else {
        let projection = arena::project(&config, &players, &awarded)
            .map_err(|error| invalid_persisted(&format!("Arena facts are invalid: {error}")))?;
        (finished_snapshot(&state, &projection), projection.players)
    };
    let results = projected_arena_results(&terminals)?;
    let games = state
        .games
        .iter()
        .map(|game| ArenaProjectionGame {
            game: game.game.clone(),
            ordinal: game.ordinal,
        })
        .collect();
    let players = metrics
        .iter()
        .zip(&state.memberships)
        .map(|(metrics, membership)| ArenaProjectionPlayer {
            membership: membership.record().clone(),
            metrics: *metrics,
        })
        .collect();
    Ok(ArenaFactsProjection {
        standings,
        results,
        games,
        players,
    })
}

fn projected_arena_results(
    terminals: &[ArenaTerminalFact<'_>],
) -> Result<Vec<ArenaResultSnapshot>, DbError> {
    let mut results = Vec::with_capacity(terminals.len());
    for terminal in terminals {
        let fact = terminal.native;
        let arena_game = terminal.game;
        let award = arena_game
            .award
            .ok_or_else(|| invalid_persisted("Arena terminal is missing its persisted award"))?;
        let game = &arena_game.game;
        let white = fact.pairing.white();
        let (outcome, no_start_absent) =
            match fact.result {
                ArenaTerminalResult::Played(outcome) => (GameOutcome::Played(outcome), None),
                ArenaTerminalResult::NoStart { absent } => {
                    let (white_result, black_result) = if absent == white {
                        (
                            AdjudicatedSideResult::ForfeitLoss,
                            AdjudicatedSideResult::ForfeitWin,
                        )
                    } else {
                        (
                            AdjudicatedSideResult::ForfeitWin,
                            AdjudicatedSideResult::ForfeitLoss,
                        )
                    };
                    let adjudicated = AdjudicatedGameOutcome::new(white_result, black_result)
                        .map_err(|error| DbError::InternalError {
                            reason: format!("could not construct Arena no-start outcome: {error}"),
                        })?;
                    (
                        GameOutcome::Adjudicated(adjudicated),
                        Some(if absent == white {
                            game.white_id
                        } else {
                            game.black_id
                        }),
                    )
                }
            };
        results.push(ArenaResultSnapshot {
            ordinal: arena_game.ordinal,
            pairing: fact.pairing,
            outcome,
            points: award.awarded_points,
            doubled: award.doubled,
            ratings: terminal.ratings.map(Some),
            no_start_absent,
        });
    }
    Ok(results)
}

pub(super) fn finished_snapshot(state: &ArenaDbState, projection: &ArenaProjection) -> Snapshot {
    let groups = projection
        .standings
        .groups
        .iter()
        .map(|group| {
            let rows = group
                .players
                .iter()
                .map(|standing| {
                    let membership = &state.memberships[standing.player.index()];
                    let metrics = &projection.players[standing.player.index()];
                    snapshot_row(membership.user_id, metrics)
                })
                .collect();
            Group {
                placement: Placement::CompetitionRank(group.rank),
                rows,
                separated_by: None,
            }
        })
        .collect();
    Snapshot { groups }
}

fn snapshot_row(user_id: Uuid, metrics: &ArenaStandingMetrics) -> Row {
    let points = Score::new(metrics.points.value());
    let performance = shared_performance(metrics);
    let arena_rating = metrics.arena_rating.map_or(Value::NotApplicable, |rating| {
        Value::SignedInteger(i64::from(rating))
    });
    Row {
        user_id,
        primary_score: Value::Score(points),
        games_played: metrics.games_played,
        matches_played: None,
        wins: metrics.wins,
        draws: metrics.draws,
        losses: metrics.losses,
        counts: vec![metrics.games_scored, metrics.berserks],
        values: vec![
            Value::Score(points),
            performance.map_or(Value::NotApplicable, Value::SignedInteger),
            arena_rating,
        ],
    }
}

fn shared_performance(metrics: &ArenaStandingMetrics) -> Option<i64> {
    metrics.performance.and_then(|performance| {
        rounded_signed_value(performance.numerator(), performance.denominator())
    })
}
