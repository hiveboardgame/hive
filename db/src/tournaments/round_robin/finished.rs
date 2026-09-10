use shared_types::tournament::standings::{Group, Placement, Row, Snapshot, Value};

use super::{RoundRobinFactsProjection, TournamentState};

pub(crate) fn round_robin_finished_snapshot(
    state: &TournamentState,
    projection: &RoundRobinFactsProjection,
) -> Snapshot {
    let mut groups = Vec::with_capacity(projection.projection.standings.groups.len());
    for group in &projection.projection.standings.groups {
        let competition_rank = group.rank;
        let mut rows = Vec::with_capacity(group.players.len());
        for standing in &group.players {
            let summary = &projection.projection.players[standing.player.index()];
            let user_id = state.memberships[standing.player.index()].user_id;
            let primary_score = summary.primary_score;
            let values = standing.values.iter().copied().map(Value::from).collect();
            rows.push(Row {
                user_id,
                primary_score: Value::Score(primary_score),
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
                values,
            });
        }
        groups.push(Group {
            placement: Placement::CompetitionRank(competition_rank),
            rows,
            separated_by: group.separated_by,
        });
    }

    Snapshot { groups }
}
