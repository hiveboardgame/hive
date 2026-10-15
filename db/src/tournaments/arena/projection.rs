use crate::{
    db_error::DbError,
    models::{Tournament, TournamentFinalOutcome},
    tournaments::{
        projection::{
            compact_game,
            rounded_signed_rating,
            rounded_signed_value,
            rounded_unsigned_rating,
        },
        public::TournamentProjection,
    },
    DbConn,
};
use shared_types::{
    tournament::{
        arena::PairingIntent,
        standings::{Group, Placement, Row, Snapshot, Value},
        Score,
    },
    tournament_view::{
        ArenaGameResponse,
        ArenaPlayerStatsResponse,
        ArenaState,
        TournamentFormatResponse,
    },
    GameId,
    TournamentStatus,
};
use std::collections::HashMap;
use tournamint::{
    arena::{self, ArenaProjection, ArenaStandingMetrics, ArenaTerminalResult},
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

pub(crate) async fn arena_projection_for_snapshot(
    tournament: &Tournament,
    selected_games: Option<&[GameId]>,
    selected_players: Option<&[Uuid]>,
    conn: &mut DbConn<'_>,
) -> Result<TournamentProjection, DbError> {
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
    let config = &state.configuration.rules;
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
        let metrics = arena::player_metrics(config, &players, &awarded)
            .map_err(|error| invalid_persisted(&format!("Arena facts are invalid: {error}")))?;
        (frozen.standings, metrics)
    } else {
        let projection = arena::project(config, &players, &awarded)
            .map_err(|error| invalid_persisted(&format!("Arena facts are invalid: {error}")))?;
        (finished_snapshot(&state, &projection), projection.players)
    };
    let mut opponent_ratings = vec![(0_u64, 0_u32); state.memberships.len()];
    for terminal in &terminals {
        let white = terminal.native.pairing.white().index();
        let black = terminal.native.pairing.black().index();
        for (player, rating) in [(white, terminal.ratings[1]), (black, terminal.ratings[0])] {
            opponent_ratings[player].0 = opponent_ratings[player]
                .0
                .checked_add(u64::from(rating))
                .ok_or_else(|| invalid_persisted("opponent rating total overflowed"))?;
            opponent_ratings[player].1 = opponent_ratings[player]
                .1
                .checked_add(1)
                .ok_or_else(|| invalid_persisted("rated game count overflowed"))?;
        }
    }
    let player_stats = metrics
        .iter()
        .zip(&state.memberships)
        .zip(opponent_ratings)
        .filter(|((_, membership), _)| {
            selected_players.is_none_or(|ids| ids.contains(&membership.user_id))
        })
        .map(|((metrics, membership), (total, count))| {
            let performance = metrics.performance;
            let stats = ArenaPlayerStatsResponse {
                player: membership.user_id,
                performance_rating: performance
                    .map(|p| rounded_signed_rating(p.numerator(), p.denominator()))
                    .transpose()?
                    .flatten(),
                average_opponent_rating: rounded_unsigned_rating(total, count)?,
                performance_games: performance.map_or(0, |p| p.denominator()),
                arena_rating: metrics.arena_rating,
                games_scored: metrics.games_scored,
                wins: metrics.wins,
                draws: metrics.draws,
                losses: metrics.losses,
                on_fire: metrics.on_fire,
                berserks: metrics.berserks,
                paused: matches!(membership.intent, PairingIntent::Paused),
            };
            Ok((stats.player, stats))
        })
        .collect::<Result<HashMap<_, _>, DbError>>()?;
    let terminals_by_ordinal = terminals
        .iter()
        .map(|terminal| (terminal.game.ordinal, terminal))
        .collect::<HashMap<_, _>>();
    let games = state
        .games
        .iter()
        .filter(|game| {
            selected_games.is_none_or(|ids| ids.iter().any(|id| id.0 == game.game.nanoid))
        })
        .map(|game| {
            let terminal = terminals_by_ordinal.get(&game.ordinal);
            let award = terminal
                .map(|terminal| {
                    terminal.game.award.ok_or_else(|| {
                        invalid_persisted("Arena terminal is missing its persisted award")
                    })
                })
                .transpose()?;
            let game_response = ArenaGameResponse {
                ordinal: game.ordinal,
                game: compact_game(&game.game)?,
                outcome: terminal
                    .map(|terminal| terminal_outcome(terminal))
                    .transpose()?,
                awarded_points: award.map(|award| award.awarded_points),
                doubled: award.map(|award| award.doubled),
            };
            Ok((game_response.game.game_id.clone(), game_response))
        })
        .collect::<Result<HashMap<_, _>, DbError>>()?;
    let featured_game_id = tournament.featured_game_id.and_then(|id| {
        state
            .games
            .iter()
            .find(|game| game.game.id == id)
            .map(|game| GameId(game.game.nanoid.clone()))
    });
    Ok(TournamentProjection {
        standings: Some(standings),
        player_stats: Vec::new(),
        format: TournamentFormatResponse::Arena(ArenaState {
            configuration: state.configuration,
            games,
            player_stats,
            featured_game_id,
        }),
    })
}

fn terminal_outcome(terminal: &ArenaTerminalFact<'_>) -> Result<GameOutcome, DbError> {
    match terminal.native.result {
        ArenaTerminalResult::Played(outcome) => Ok(GameOutcome::Played(outcome)),
        ArenaTerminalResult::NoStart { absent } => {
            let (white, black) = if absent == terminal.native.pairing.white() {
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
            AdjudicatedGameOutcome::new(white, black)
                .map(GameOutcome::Adjudicated)
                .map_err(|error| {
                    invalid_persisted(&format!(
                        "could not construct Arena no-start outcome: {error}"
                    ))
                })
        }
    }
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
