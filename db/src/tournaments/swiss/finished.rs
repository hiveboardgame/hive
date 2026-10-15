use shared_types::tournament::{
    standings::{Group, Placement, Row, Snapshot, Value},
    Format,
};

use super::{SwissFactsProjection, TournamentState};
use tournamint::{swiss::SwissProjection, PlayerId};
use uuid::Uuid;

pub(crate) fn swiss_finished_snapshot(
    state: &TournamentState,
    projected: &SwissFactsProjection,
) -> Snapshot {
    swiss_standings_snapshot(
        &projected.projection,
        state.configuration.format() == Format::DoubleSwiss,
        |player| state.memberships[player.index()].user_id,
    )
}

pub fn swiss_standings_snapshot(
    projection: &SwissProjection,
    double_swiss: bool,
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
