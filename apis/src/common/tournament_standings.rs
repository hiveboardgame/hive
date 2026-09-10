use shared_types::tournament::standings::{Placement, Row, Snapshot};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct DisplayStandingRow {
    pub(crate) rank: Option<u32>,
    pub(crate) row: Row,
    pub(crate) separated_by: Option<u32>,
    pub(crate) withdrawn: bool,
}

pub(crate) fn display_standing_rows(
    snapshot: &Snapshot,
    withdrawn_players: &HashSet<Uuid>,
) -> Vec<DisplayStandingRow> {
    let mut ranked = Vec::new();
    let mut withdrawn = Vec::new();
    let mut next_rank = 1_u32;

    for group in &snapshot.groups {
        let active_rows = group
            .rows
            .iter()
            .filter(|row| !withdrawn_players.contains(&row.user_id))
            .cloned()
            .collect::<Vec<_>>();
        if !active_rows.is_empty() {
            let rank = match group.placement {
                Placement::CompetitionRank(_) => Some(next_rank),
                Placement::EliminationTier(rank) => Some(rank),
            };
            next_rank = next_rank.saturating_add(active_rows.len() as u32);
            ranked.extend(active_rows.into_iter().enumerate().map(|(index, row)| {
                DisplayStandingRow {
                    rank,
                    row,
                    separated_by: if index == 0 { group.separated_by } else { None },
                    withdrawn: false,
                }
            }));
        }

        withdrawn.extend(
            group
                .rows
                .iter()
                .filter(|row| withdrawn_players.contains(&row.user_id))
                .cloned()
                .map(|row| DisplayStandingRow {
                    rank: None,
                    row,
                    separated_by: None,
                    withdrawn: true,
                }),
        );
    }

    ranked.extend(withdrawn);
    ranked
}
