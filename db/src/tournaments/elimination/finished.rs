use shared_types::tournament::{
    standings::{Group, Placement, Row, Snapshot, Value},
    Resolution,
};
use std::collections::{BTreeMap, HashMap};
use tournamint::{
    elimination::{EliminationNodeFactState, EliminationNodeResolution, EliminationProjection},
    MatchScore,
    PlayerId,
};

use super::{EliminationFactsProjection, TournamentState};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct EliminationPlayerCounts {
    pub(crate) games_played: u32,
    pub(crate) matches_played: u32,
    pub(crate) wins: u32,
    pub(crate) draws: u32,
    pub(crate) losses: u32,
}

pub(crate) fn elimination_finished_snapshot(
    state: &TournamentState,
    projected: &EliminationFactsProjection,
) -> Snapshot {
    let fact_projection = &projected.projection;
    let counts = elimination_player_counts(state, fact_projection);
    let placements = fact_projection.players.iter().fold(
        BTreeMap::<u32, Vec<PlayerId>>::new(),
        |mut placements, result| {
            if let Some(place) = result.placement {
                placements.entry(place).or_default().push(result.player);
            }
            placements
        },
    );
    let mut groups = Vec::with_capacity(placements.len());
    for (place, mut players) in placements {
        players.sort_by_key(|player| player.index());
        let mut rows = Vec::with_capacity(players.len());
        for player in players {
            let user_id = state.memberships[player.index()].user_id;
            let player_counts = counts[player.index()];
            let placement_value = Value::Integer(u64::from(place));
            rows.push(Row {
                user_id,
                primary_score: placement_value,
                games_played: player_counts.games_played,
                matches_played: Some(player_counts.matches_played),
                wins: player_counts.wins,
                draws: player_counts.draws,
                losses: player_counts.losses,
                counts: Vec::new(),
                values: vec![placement_value],
            });
        }
        groups.push(Group {
            placement: Placement::EliminationTier(place),
            rows,
            separated_by: None,
        });
    }
    Snapshot { groups }
}

pub(crate) fn elimination_player_counts(
    state: &TournamentState,
    projection: &EliminationProjection,
) -> Vec<EliminationPlayerCounts> {
    let mut counts = vec![EliminationPlayerCounts::default(); state.memberships.len()];
    let games_by_slot = state.games_by_slot();
    let players = state
        .memberships
        .iter()
        .enumerate()
        .map(|(index, membership)| (membership.user_id, PlayerId::new(index)))
        .collect::<HashMap<_, _>>();
    for slot in &state.slots {
        let Some(resolution) = slot.resolution else {
            continue;
        };
        let outcome = match resolution {
            Resolution::Result(outcome) => outcome,
            Resolution::Withdrawal(_) | Resolution::Clinched => continue,
        };
        let white = players[&slot.white];
        let black = players[&slot.black];
        if games_by_slot
            .get(&slot.id)
            .is_some_and(|game| game.last_interaction.is_some())
        {
            counts[white.index()].games_played += 1;
            counts[black.index()].games_played += 1;
        }
        record_result(&mut counts[white.index()], outcome.white().score());
        record_result(&mut counts[black.index()], outcome.black().score());
    }
    for projected_node in &projection.nodes {
        let EliminationNodeFactState::Resolved {
            resolution: EliminationNodeResolution::MatchDecided { winner, loser },
            ..
        } = projected_node.state
        else {
            continue;
        };
        for player in [winner, loser] {
            counts[player.index()].matches_played += 1;
        }
    }
    counts
}

fn record_result(counts: &mut EliminationPlayerCounts, result: MatchScore) {
    let count = match result {
        MatchScore::Win => &mut counts.wins,
        MatchScore::Draw => &mut counts.draws,
        MatchScore::Loss => &mut counts.losses,
    };
    *count += 1;
}
