use shared_types::tournament::standings::{Group, Placement, Row, Snapshot, Value};

use super::{RoundRobinFactsProjection, TournamentState};
use tournamint::{round_robin::RoundRobinProjection, PlayerId};
use uuid::Uuid;

pub(crate) fn round_robin_finished_snapshot(
    state: &TournamentState,
    projection: &RoundRobinFactsProjection,
) -> Snapshot {
    round_robin_standings_snapshot(&projection.projection, |player| {
        state.memberships[player.index()].user_id
    })
}

pub fn round_robin_standings_snapshot(
    projection: &RoundRobinProjection,
    user_id: impl Fn(PlayerId) -> Uuid,
) -> Snapshot {
    let mut groups = Vec::with_capacity(projection.standings.groups.len());
    for group in &projection.standings.groups {
        let competition_rank = group.rank;
        let mut rows = Vec::with_capacity(group.players.len());
        for standing in &group.players {
            let summary = &projection.players[standing.player.index()];
            let user_id = user_id(standing.player);
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
