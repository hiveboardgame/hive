use leptos::prelude::*;
use shared_types::tournament::standings::Snapshot;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ComparisonMember {
    pub user_id: Uuid,
    pub place: usize,
    pub tied: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DirectEncounterValue {
    pub members: Arc<Vec<ComparisonMember>>,
    pub member_index: usize,
}

impl DirectEncounterValue {
    pub fn text(&self) -> String {
        let member = &self.members[self.member_index];
        let suffix = match (member.place % 100, member.place % 10) {
            (11..=13, _) => "th",
            (_, 1) => "st",
            (_, 2) => "nd",
            (_, 3) => "rd",
            _ => "th",
        };
        let equal = if member.tied { "=" } else { "" };
        // TODO: i18n once copy is approved.
        format!("{equal}{}{suffix} of {}", member.place, self.members.len())
    }

    fn starts_partition(&self) -> bool {
        self.member_index > 0
            && self.members[self.member_index - 1].place != self.members[self.member_index].place
    }
}

pub(super) fn direct_encounter_values(
    snapshot: &Snapshot,
    criterion_index: usize,
) -> HashMap<Uuid, DirectEncounterValue> {
    let mut values = HashMap::new();
    // Earlier boundaries delimit the group entering this criterion. Later
    // boundaries must stay joined: they cannot change this step's tied places.
    for group in snapshot.groups.chunk_by(|_, next| {
        !next
            .separated_by
            .is_some_and(|index| (index as usize) < criterion_index)
    }) {
        let separates =
            |index: Option<u32>| index.is_some_and(|index| index as usize == criterion_index);
        if !group
            .iter()
            .skip(1)
            .any(|group| separates(group.separated_by))
        {
            continue;
        }

        let mut members = Vec::new();
        for partition in group.chunk_by(|_, next| !separates(next.separated_by)) {
            let place = members.len() + 1;
            let tied = partition
                .iter()
                .map(|group| group.rows.len())
                .sum::<usize>()
                > 1;
            members.extend(partition.iter().flat_map(|group| &group.rows).map(|row| {
                ComparisonMember {
                    user_id: row.user_id,
                    place,
                    tied,
                }
            }));
        }
        let members = Arc::new(members);
        for (member_index, member) in members.iter().enumerate() {
            values.insert(
                member.user_id,
                DirectEncounterValue {
                    members: members.clone(),
                    member_index,
                },
            );
        }
    }
    values
}

#[component]
pub(super) fn DirectEncounterCell(value: DirectEncounterValue) -> impl IntoView {
    let text = value.text();
    let starts_group = value.member_index == 0;
    let ends_group = value.member_index + 1 == value.members.len();
    let starts_partition = value.starts_partition();
    view! {
        <td class="p-0 text-sm text-center whitespace-nowrap">
            <div class="relative py-2 px-2 bg-blue-50 border-blue-200 border-x dark:border-sky-800 dark:bg-sky-950">
                {starts_group
                    .then(|| {
                        view! {
                            <span
                                aria-hidden="true"
                                class="absolute top-0 -inset-x-px h-0.5 border-b border-blue-200 bg-even-light dark:border-sky-800 dark:bg-surface-panel"
                            ></span>
                        }
                    })}
                {starts_partition
                    .then(|| {
                        view! {
                            <span
                                aria-hidden="true"
                                class="absolute inset-x-0 top-0 h-0.5 bg-blue-400 dark:bg-sky-500"
                            ></span>
                        }
                    })} {text}
                {ends_group
                    .then(|| {
                        view! {
                            <span
                                aria-hidden="true"
                                class="absolute bottom-0 -inset-x-px h-0.5 border-t border-blue-200 bg-even-light dark:border-sky-800 dark:bg-surface-panel"
                            ></span>
                        }
                    })}
            </div>
        </td>
    }
}

#[cfg(test)]
mod tests {
    use super::direct_encounter_values;
    use shared_types::tournament::standings::{Group, Placement, Row, Snapshot, Value};
    use uuid::Uuid;

    fn snapshot(boundaries: &[(Option<u32>, &[u128])]) -> Snapshot {
        let mut rank = 1;
        Snapshot {
            groups: boundaries
                .iter()
                .map(|(separated_by, players)| {
                    let group = Group {
                        placement: Placement::CompetitionRank(rank),
                        separated_by: *separated_by,
                        rows: players
                            .iter()
                            .map(|id| Row {
                                user_id: Uuid::from_u128(*id),
                                primary_score: Value::Integer(0),
                                games_played: 0,
                                matches_played: None,
                                wins: 0,
                                draws: 0,
                                losses: 0,
                                counts: Vec::new(),
                                values: Vec::new(),
                            })
                            .collect(),
                    };
                    rank += players.len() as u32;
                    group
                })
                .collect(),
        }
    }

    #[test]
    fn criterion_boundaries_preserve_ties_until_the_step_that_separates_them() {
        let snapshot = snapshot(&[
            (None, &[1]),
            (Some(0), &[2]),
            (Some(3), &[3]),
            (Some(2), &[4]),
            (Some(3), &[5]),
            (Some(1), &[6]),
        ]);
        let values = direct_encounter_values(&snapshot, 2);
        assert_eq!(values.len(), 4);
        for (id, place) in [(2, 1), (3, 1), (4, 3), (5, 3)] {
            let value = &values[&Uuid::from_u128(id)];
            let member = &value.members[value.member_index];
            assert_eq!(value.members.len(), 4);
            assert_eq!(member.place, place);
            assert!(member.tied);
        }
        assert!(!values[&Uuid::from_u128(3)].starts_partition());
        assert!(values[&Uuid::from_u128(4)].starts_partition());

        let later = direct_encounter_values(&snapshot, 3);
        for (id, place) in [(2, 1), (3, 2), (4, 1), (5, 2)] {
            let value = &later[&Uuid::from_u128(id)];
            assert_eq!(value.members.len(), 2);
            assert_eq!(value.members[value.member_index].place, place);
            assert!(!value.members[value.member_index].tied);
        }
    }

    #[test]
    fn each_comparison_retains_its_full_membership_and_recursive_partition_outcome() {
        let snapshot = snapshot(&[
            (None, &[1]),
            (Some(1), &[2]),
            (Some(1), &[3, 4]),
            (Some(0), &[5]),
            (Some(1), &[6]),
            (Some(0), &[7]),
            (Some(2), &[8]),
        ]);
        let values = direct_encounter_values(&snapshot, 1);
        assert_eq!(values.len(), 6);
        let fourth = &values[&Uuid::from_u128(4)];
        assert_eq!(
            fourth
                .members
                .iter()
                .map(|member| (member.user_id.as_u128(), member.place, member.tied,))
                .collect::<Vec<_>>(),
            vec![(1, 1, false), (2, 2, false), (3, 3, true), (4, 3, true)]
        );
        let sixth = &values[&Uuid::from_u128(6)];
        assert_eq!(
            sixth
                .members
                .iter()
                .map(|member| member.user_id.as_u128())
                .collect::<Vec<_>>(),
            vec![5, 6]
        );
        assert_eq!(sixth.members[sixth.member_index].place, 2);
        assert!(!values.contains_key(&Uuid::from_u128(7)));
        assert!(!values.contains_key(&Uuid::from_u128(8)));
    }
}
