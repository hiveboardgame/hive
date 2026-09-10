use super::*;
use crate::common::half_point_text;
use tournamint::PointSystem;

#[derive(Clone, PartialEq)]
enum StandardHistoryRow {
    Result {
        game: HistoryGame,
        opponent: OpponentLabel,
        ordinal: Option<usize>,
    },
    Group {
        games: Vec<HistoryGame>,
        opponent: OpponentLabel,
        headline_score: String,
        ordinal: Option<usize>,
        expanded_score: Option<String>,
    },
}

fn result_class(result: Option<MatchScore>) -> &'static str {
    match result {
        Some(MatchScore::Win) => "text-pillbug-teal",
        Some(MatchScore::Draw) | None => "text-gray-600 dark:text-gray-300",
        Some(MatchScore::Loss) => "text-ladybug-red",
    }
}

#[component]
fn CompactGameRow(game: HistoryGame, label: String) -> impl IntoView {
    let color = game.color;
    let state = game.state;
    let result = game.board_result;
    let result_class = result_class(game.result);
    let game_id = game.game_id;
    let content = view! {
        <span class="min-w-0">
            <span class="block font-medium truncate">{label}</span>
            {state
                .map(|state| {
                    view! {
                        <span class="block text-xs text-gray-500 dark:text-gray-400">{state}</span>
                    }
                })}
        </span>
        <span class="inline-flex justify-center w-5">
            {color
                .map(|color| {
                    view! { <ColorHex color=Signal::derive(move || color) /> }
                })}
        </span>
        <span class=format!("font-semibold tabular-nums text-right {result_class}")>{result}</span>
    };
    let row_class = player_history_row_class("grid-cols-[minmax(0,1fr)_1.25rem_3.5rem]");
    match game_id {
        Some(game_id) => view! {
            <a class=format!("{row_class} no-link-style") href=format!("/game/{game_id}")>
                {content}
            </a>
        }
        .into_any(),
        None => view! { <div class=row_class>{content}</div> }.into_any(),
    }
}

#[component]
fn CompactResultRow(
    game: HistoryGame,
    opponent: OpponentLabel,
    ordinal: Option<usize>,
) -> impl IntoView {
    let color = game.color;
    let score = game.selected_score;
    let state = game.state;
    let result_class = result_class(game.result);
    let game_id = game.game_id;
    let row_class = if ordinal.is_some() {
        player_history_row_class("grid-cols-[2rem_minmax(0,1fr)_1.25rem_3rem]")
    } else {
        player_history_row_class("grid-cols-[minmax(0,1fr)_1.25rem_3rem]")
    };
    let content = view! {
        {ordinal
            .map(|ordinal| {
                view! {
                    <span class="font-semibold tabular-nums text-center text-gray-500 dark:text-gray-400">
                        {ordinal}
                    </span>
                }
            })}
        <span class="min-w-0">
            <span class="flex gap-1 items-baseline min-w-0">
                <span class="min-w-0 font-semibold truncate" title=opponent.name.clone()>
                    {opponent.name.clone()}
                </span>
                {opponent
                    .rating
                    .map(|rating| {
                        view! {
                            <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                "· " {rating}
                            </span>
                        }
                    })}
            </span>
            {state
                .map(|state| {
                    view! {
                        <span class="block text-xs text-gray-500 dark:text-gray-400">{state}</span>
                    }
                })}
        </span>
        <span class="inline-flex justify-center w-5">
            {color
                .map(|color| {
                    view! { <ColorHex color=Signal::derive(move || color) /> }
                })}
        </span>
        <span class=format!("font-semibold tabular-nums text-right {result_class}")>{score}</span>
    };
    match game_id {
        Some(game_id) => view! {
            <a class=format!("{row_class} no-link-style") href=format!("/game/{game_id}")>
                {content}
            </a>
        }
        .into_any(),
        None => view! { <div class=row_class>{content}</div> }.into_any(),
    }
}

#[component]
fn ExpandableHistoryGroup(
    games: Vec<HistoryGame>,
    opponent: OpponentLabel,
    headline_score: String,
    ordinal: Option<usize>,
    expanded_score: Option<String>,
) -> impl IntoView {
    let child_rows = games
        .into_iter()
        .enumerate()
        .map(|(fallback_index, game)| {
            let number = if game.order == usize::MAX {
                fallback_index.saturating_add(1)
            } else {
                game.order.saturating_add(1)
            };
            // TODO: i18n once copy is approved.
            let label = format!("Game {number}");
            view! { <CompactGameRow game label /> }
        })
        .collect_view();
    let summary_class = if ordinal.is_some() {
        "grid grid-cols-[2rem_minmax(0,1fr)_3.5rem_1rem] gap-1 items-center"
    } else {
        "grid grid-cols-[minmax(0,1fr)_3.5rem_1rem] gap-1 items-center"
    };
    view! {
        <details class=PLAYER_HISTORY_GROUP_SHELL>
            <summary class="py-1.5 px-1 list-none cursor-pointer [&::-webkit-details-marker]:hidden dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70">
                <span class=summary_class>
                    {ordinal
                        .map(|ordinal| {
                            view! {
                                <span class="font-semibold tabular-nums text-center text-gray-500 dark:text-gray-400">
                                    {ordinal}
                                </span>
                            }
                        })} <span class="flex gap-1 items-baseline min-w-0">
                        <span class="min-w-0 font-semibold truncate" title=opponent.name.clone()>
                            {opponent.name.clone()}
                        </span>
                        {opponent
                            .rating
                            .map(|rating| {
                                view! {
                                    <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                        "· " {rating}
                                    </span>
                                }
                            })}
                    </span>
                    <span class="font-semibold tabular-nums text-right">{headline_score}</span>
                    <Icon
                        icon=icondata_lu::LuChevronDown
                        attr:class="text-gray-600 transition-transform size-4 dark:text-gray-300 group-open:rotate-180"
                    />
                </span>
            </summary>
            <div class="border-t border-black/5 dark:border-white/5">
                {expanded_score
                    .map(|expanded_score| {
                        view! {
                            <p class="py-1 px-3 text-sm font-semibold tabular-nums text-right text-gray-600 dark:text-gray-300">
                                {expanded_score}
                            </p>
                        }
                    })} <div>{child_rows}</div>
            </div>
        </details>
    }
}

fn round_robin_history(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    player: Uuid,
) -> Vec<StandardHistoryRow> {
    let configuration = FormatConfig::RoundRobin(round_robin.configuration().get());
    let rounds = round_robin.rounds().get();
    let matches = round_robin.matches().get();
    struct Group {
        opponent: Uuid,
        latest_position: (usize, usize, usize, Uuid),
        games: Vec<HistoryGame>,
        own_points: u32,
        opponent_points: u32,
        has_award: bool,
    }

    let mut groups = Vec::<Group>::new();
    for round in &rounds {
        for item in &round.slots {
            if !round_robin
                .slots()
                .with_untracked(|slots| slots.contains_key(&item.slot_id))
            {
                continue;
            }
            let slot = round_robin.slots().at_key(item.slot_id).get();
            if !slot.participants.contains(&player) {
                continue;
            }
            let Some(opponent) = slot_opponent(&slot, player) else {
                continue;
            };
            let side = slot_side(&slot, player).unwrap_or_default();
            let position = (
                round.round_index,
                round.pass_index,
                item.board_index,
                slot.id,
            );
            let game = slot_history_game(&configuration, &slot, player, round.pass_index);
            if let Some(group) = groups.iter_mut().find(|group| group.opponent == opponent) {
                if position > group.latest_position {
                    group.latest_position = position;
                }
                group.games.push(game);
                if let Some(points) = slot.awarded_game_points {
                    group.own_points = group.own_points.saturating_add(points[side].value());
                    group.opponent_points = group
                        .opponent_points
                        .saturating_add(points[1 - side].value());
                    group.has_award = true;
                }
            } else {
                let (own_points, opponent_points, has_award) =
                    slot.awarded_game_points.map_or((0, 0, false), |points| {
                        (points[side].value(), points[1 - side].value(), true)
                    });
                groups.push(Group {
                    opponent,
                    latest_position: position,
                    games: vec![game],
                    own_points,
                    opponent_points,
                    has_award,
                });
            }
        }
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.latest_position));
    groups
        .into_iter()
        .filter_map(|group| {
            let opponent = opponent_label(common, &configuration, Some(group.opponent));
            if group.games.len() == 1 {
                return group
                    .games
                    .into_iter()
                    .next()
                    .map(|game| StandardHistoryRow::Result {
                        game,
                        opponent,
                        ordinal: None,
                    });
            }
            let headline_score = if group.has_award {
                format!(
                    "{}–{}",
                    game_points_text(&configuration, Score::new(group.own_points)),
                    game_points_text(&configuration, Score::new(group.opponent_points)),
                )
            } else {
                String::from("—")
            };
            let (headline_score, expanded_score) = if matches!(&configuration, FormatConfig::RoundRobin(config)
                if config.primary_score == RoundRobinPrimaryScore::MatchPoints) {
                let match_points = matches.iter().find(|encounter| {
                    encounter.participants.contains(&player) && encounter.participants.contains(&group.opponent)
                }).and_then(|encounter| encounter.completion.map(|completed| {
                    let side = usize::from(encounter.participants[1] == player);
                    format!("{}–{}", primary_score_text(&configuration, completed.match_points[side]),
                        primary_score_text(&configuration, completed.match_points[1 - side]))
                })).unwrap_or_else(|| String::from("—"));
                // TODO: i18n once copy is approved.
                (match_points, Some(format!("Game points: {headline_score}")))
            } else { (headline_score, None) };
            Some(StandardHistoryRow::Group {
                games: group.games,
                opponent,
                headline_score,
                ordinal: None,
                expanded_score,
            })
        })
        .collect()
}

fn swiss_bye_game(
    configuration: &FormatConfig,
    game_points: Score,
    match_points: Score,
) -> HistoryGame {
    let selected_score = match configuration {
        FormatConfig::Swiss(swiss_configuration)
            if matches!(
                swiss_configuration.system,
                SwissSystem::DoubleSwiss(system)
                    if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
            ) =>
        {
            primary_score_text(configuration, match_points)
        }
        _ => game_points_text(configuration, game_points),
    };
    empty_history_game(selected_score, None)
}

fn player_game_points(
    slots: &[SlotResponse],
    player: Uuid,
    point_system: PointSystem,
) -> Option<[Score; 2]> {
    let mut totals = [0u32; 2];
    let mut has_points = false;
    for slot in slots {
        let Some(side) = slot_side(slot, player) else {
            continue;
        };
        let Some(points) = slot.awarded_game_points.or_else(|| {
            slot.outcome().map(|outcome| {
                [
                    point_system.points_for(outcome.white()),
                    point_system.points_for(outcome.black()),
                ]
            })
        }) else {
            continue;
        };
        totals[0] = totals[0].checked_add(points[side].value())?;
        totals[1] = totals[1].checked_add(points[1 - side].value())?;
        has_points = true;
    }
    has_points.then(|| totals.map(Score::new))
}

fn swiss_history(
    common: Store<TournamentCommon>,
    swiss: Store<SwissState>,
    player: Uuid,
) -> Vec<StandardHistoryRow> {
    let swiss_configuration = swiss.configuration().get();
    let configuration = FormatConfig::Swiss(swiss_configuration.clone());
    let rounds = swiss.rounds().get();
    let mut history = Vec::<(u32, StandardHistoryRow)>::new();
    for round in &rounds {
        for encounter in round
            .encounters
            .iter()
            .filter(|encounter| encounter.participants.contains(&player))
        {
            let side = usize::from(encounter.participants[1] == player);
            let opponent = opponent_label(
                common,
                &configuration,
                Some(encounter.participants[1 - side]),
            );
            match &swiss_configuration.system {
                SwissSystem::Dutch(_) => {
                    if let Some(slot_id) = encounter.slot_ids.first() {
                        if swiss
                            .slots()
                            .with_untracked(|slots| slots.contains_key(slot_id))
                        {
                            let slot = swiss.slots().at_key(*slot_id).get();
                            history.push((
                                round.round_index,
                                StandardHistoryRow::Result {
                                    game: slot_history_game(&configuration, &slot, player, 0),
                                    opponent,
                                    ordinal: Some(round.round_index as usize + 1),
                                },
                            ));
                        }
                    }
                }
                SwissSystem::DoubleSwiss(system) => {
                    let slots = encounter
                        .slot_ids
                        .iter()
                        .filter(|slot_id| {
                            swiss
                                .slots()
                                .with_untracked(|slots| slots.contains_key(slot_id))
                        })
                        .map(|slot_id| swiss.slots().at_key(*slot_id).get())
                        .collect::<Vec<_>>();
                    let games = slots
                        .iter()
                        .enumerate()
                        .map(|(order, slot)| slot_history_game(&configuration, slot, player, order))
                        .collect::<Vec<_>>();
                    let headline_score = encounter
                        .completion
                        .map(|completion| {
                            if system.primary_score == DoubleSwissPrimaryScore::MatchPoints {
                                primary_score_text(&configuration, completion.match_points[side])
                            } else {
                                game_points_text(&configuration, completion.game_points[side])
                            }
                        })
                        .unwrap_or_else(|| String::from("—"));
                    let expanded_score = (system.primary_score
                        == DoubleSwissPrimaryScore::MatchPoints)
                        .then(|| {
                            player_game_points(
                                &slots,
                                player,
                                swiss_configuration.game_point_system,
                            )
                        })
                        .flatten()
                        .map(|points| {
                            // TODO: i18n once copy is approved.
                            format!(
                                "Game points: {}–{}",
                                game_points_text(&configuration, points[0]),
                                game_points_text(&configuration, points[1]),
                            )
                        });
                    history.push((
                        round.round_index,
                        StandardHistoryRow::Group {
                            games,
                            opponent,
                            headline_score,
                            ordinal: Some(round.round_index as usize + 1),
                            expanded_score,
                        },
                    ));
                }
                _ => unreachable!("unsupported Swiss system passed creation validation"),
            }
        }
        for bye in round.byes.iter().filter(|bye| bye.player == player) {
            history.push((
                round.round_index,
                StandardHistoryRow::Result {
                    game: swiss_bye_game(&configuration, bye.game_points, bye.match_points),
                    opponent: opponent_label(common, &configuration, None),
                    ordinal: Some(round.round_index as usize + 1),
                },
            ));
        }
    }
    history.sort_by_key(|(round_index, _)| *round_index);
    history.reverse();
    history.into_iter().map(|(_, row)| row).collect()
}

fn elimination_history(
    common: Store<TournamentCommon>,
    elimination: Store<EliminationState>,
    player: Uuid,
) -> Vec<StandardHistoryRow> {
    let configuration = FormatConfig::Elimination(elimination.configuration().get());
    let nodes = elimination.nodes().get();
    let mut nodes = nodes
        .iter()
        .filter_map(|node| {
            let side = node
                .entrants
                .iter()
                .position(|entrant| *entrant == Some(player))?;
            (node.series.is_some()
                || matches!(
                    node.state,
                    EliminationNodeStateResponse::Resolved(
                        EliminationNodeResolution::AutomaticAdvance { player: advanced }
                    ) if advanced == player
                ))
            .then_some((node, side))
        })
        .collect::<Vec<_>>();
    nodes.sort_by_key(|(node, _)| (node.wave_index, node.stage_ordinal));
    nodes.reverse();
    let count = nodes.len();
    nodes
        .into_iter()
        .enumerate()
        .map(|(index, (node, side))| {
            let ordinal = Some(count - index);
            let Some(series) = node.series.as_ref() else {
                return StandardHistoryRow::Result {
                    game: empty_history_game(String::from("—"), None),
                    opponent: opponent_label(common, &configuration, None),
                    ordinal,
                };
            };
            let games = series
                .sets
                .iter()
                .flat_map(|set| &set.slots)
                .filter(|series_slot| {
                    elimination
                        .slots()
                        .with_untracked(|slots| slots.contains_key(&series_slot.slot_id))
                })
                .map(|series_slot| elimination.slots().at_key(series_slot.slot_id).get())
                .filter(|slot| slot.participants.contains(&player))
                .enumerate()
                .map(|(order, slot)| slot_history_game(&configuration, &slot, player, order))
                .collect::<Vec<_>>();
            let has_result = games.iter().any(|game| game.result.is_some());
            let headline_score = if has_result {
                format!(
                    "{}–{}",
                    half_point_text(series.score[side]),
                    half_point_text(series.score[1 - side]),
                )
            } else {
                String::from("—")
            };
            StandardHistoryRow::Group {
                games,
                opponent: opponent_label(common, &configuration, node.entrants[1 - side]),
                headline_score,
                ordinal,
                expanded_score: None,
            }
        })
        .collect()
}

fn sort_group_games(mut history: Vec<StandardHistoryRow>) -> Vec<StandardHistoryRow> {
    for row in &mut history {
        if let StandardHistoryRow::Group { games, .. } = row {
            games.sort_by_key(|game| game.order);
        }
    }
    history
}

fn render_history_row(row: StandardHistoryRow) -> AnyView {
    match row {
        StandardHistoryRow::Result {
            game,
            opponent,
            ordinal,
        } => view! { <CompactResultRow game opponent ordinal /> }.into_any(),
        StandardHistoryRow::Group {
            games,
            opponent,
            headline_score,
            ordinal,
            expanded_score,
        } => view! { <ExpandableHistoryGroup games opponent headline_score ordinal expanded_score /> }
        .into_any(),
    }
}

#[component]
fn StandardStartedPlayerDetailsBody(
    common: Store<TournamentCommon>,
    player: Uuid,
    history: Memo<Vec<StandardHistoryRow>>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="space-y-3">
            {move || {
                let statistics = player_statistics(common, player);
                view! { <PlayerStatisticsTable statistics berserk_rate=None /> }
            }}
            <Show
                when=move || history.with(|history| !history.is_empty())
                fallback=move || {
                    view! {
                        <p class="ui-empty-state">
                            {t!(i18n, tournaments.view.standings.no_results)}
                        </p>
                    }
                }
            >
                <div>
                    {move || { history.get().into_iter().map(render_history_row).collect_view() }}
                </div>
            </Show>
        </div>
    }
    .into_any()
}

#[component]
pub(super) fn RoundRobinPlayerDetailsBody(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    player: Uuid,
) -> impl IntoView {
    let history =
        Memo::new(move |_| sort_group_games(round_robin_history(common, round_robin, player)));
    view! { <StandardStartedPlayerDetailsBody common player history /> }
}

#[component]
pub(super) fn SwissPlayerDetailsBody(
    common: Store<TournamentCommon>,
    swiss: Store<SwissState>,
    player: Uuid,
) -> impl IntoView {
    let history = Memo::new(move |_| sort_group_games(swiss_history(common, swiss, player)));
    view! { <StandardStartedPlayerDetailsBody common player history /> }
}

#[component]
pub(super) fn EliminationPlayerDetailsBody(
    common: Store<TournamentCommon>,
    elimination: Store<EliminationState>,
    player: Uuid,
) -> impl IntoView {
    let history =
        Memo::new(move |_| sort_group_games(elimination_history(common, elimination, player)));
    view! { <StandardStartedPlayerDetailsBody common player history /> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::{tournament::SlotKey, Clock, RealtimeClock};
    use std::{collections::HashSet, num::NonZeroU32};
    use tournamint::swiss::{SwissGameId, SwissLeg};

    fn scored_slot(
        participants: [Uuid; 2],
        outcome: PlayedGameOutcome,
        leg: SwissLeg,
    ) -> SlotResponse {
        let outcome = GameOutcome::Played(outcome);
        SlotResponse {
            id: Uuid::nil(),
            key: SlotKey::Swiss {
                slot: SwissGameId {
                    round_index: 0,
                    pairing_index: 0,
                    leg,
                },
            },
            participants,
            clock: Clock::Realtime(RealtimeClock {
                base_seconds: NonZeroU32::new(300).unwrap(),
                increment_seconds: 0,
            }),
            resolution: Some(Resolution::Result(outcome)),
            outcome: Some(outcome),
            awarded_game_points: None,
            resolved_at: None,
            game: None,
            scheduled_at: None,
            deadline_at: None,
            waits_for: None,
            available_admin_actions: HashSet::new(),
        }
    }

    #[test]
    fn double_swiss_game_points_follow_each_slots_colors_and_awards() {
        let players = [Uuid::from_u128(1), Uuid::from_u128(2)];
        for (first, second, expected) in [
            (PlayedGameOutcome::Draw, PlayedGameOutcome::BlackWin, [3, 1]),
            (
                PlayedGameOutcome::WhiteWin,
                PlayedGameOutcome::BlackWin,
                [4, 0],
            ),
            (PlayedGameOutcome::Draw, PlayedGameOutcome::Draw, [2, 2]),
            (
                PlayedGameOutcome::WhiteWin,
                PlayedGameOutcome::WhiteWin,
                [2, 2],
            ),
        ] {
            let slots = [
                scored_slot(players, first, SwissLeg::First),
                scored_slot([players[1], players[0]], second, SwissLeg::Second),
            ];
            assert_eq!(
                player_game_points(&slots, players[0], PointSystem::STANDARD),
                Some(expected.map(Score::new))
            );
            assert_eq!(
                player_game_points(&slots, players[1], PointSystem::STANDARD),
                Some([expected[1], expected[0]].map(Score::new))
            );
        }
        let custom = PointSystem {
            win: Score::new(6),
            draw: Score::new(2),
            ..PointSystem::STANDARD
        };
        let mut slots = [
            scored_slot(players, PlayedGameOutcome::Draw, SwissLeg::First),
            scored_slot(
                [players[1], players[0]],
                PlayedGameOutcome::BlackWin,
                SwissLeg::Second,
            ),
        ];
        assert_eq!(
            player_game_points(&slots, players[0], custom),
            Some([8, 2].map(Score::new))
        );
        // Persisted awards may differ from the raw outcome after administrative policy.
        slots[1].awarded_game_points = Some([Score::new(1), Score::new(4)]);
        slots[1].resolution = Some(Resolution::Withdrawal(GameOutcome::Played(
            PlayedGameOutcome::BlackWin,
        )));
        assert_eq!(
            player_game_points(&slots, players[0], custom),
            Some([6, 3].map(Score::new))
        );
        assert_eq!(
            player_game_points(&slots, players[1], custom),
            Some([3, 6].map(Score::new))
        );
    }
}
