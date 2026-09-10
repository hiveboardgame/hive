use shared_types::tournament::{
    standings::{Group, Placement, Row, Snapshot, Value},
    Format,
};

use super::{SwissFactsProjection, TournamentState};

pub(crate) fn swiss_finished_snapshot(
    state: &TournamentState,
    projected: &SwissFactsProjection,
) -> Snapshot {
    let projection = &projected.projection;
    let double_swiss = state.configuration.format() == Format::DoubleSwiss;
    let mut groups = Vec::with_capacity(projection.standings.groups.len());
    for group in &projection.standings.groups {
        let competition_rank = group.rank;
        let mut rows = Vec::with_capacity(group.players.len());
        for standing in &group.players {
            let summary = &projection.players[standing.player.index()];
            let user_id = state.memberships[standing.player.index()].user_id;
            let primary_score = summary.primary_score;
            let values = standing.values.iter().copied().map(Value::from).collect();
            let mut counts = vec![summary.pairing_allocated_byes, summary.requested_byes];
            if double_swiss {
                counts.extend([
                    summary.match_wins,
                    summary.match_draws,
                    summary.match_losses,
                ]);
            }
            rows.push(Row {
                user_id,
                primary_score: Value::Score(primary_score),
                games_played: summary.games_played,
                matches_played: double_swiss.then_some(summary.matches_played),
                wins: summary.game_wins,
                draws: summary.game_draws,
                losses: summary.game_losses,
                counts,
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
