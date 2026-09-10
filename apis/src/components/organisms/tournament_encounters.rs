use crate::{
    common::{
        display_standing_rows,
        game_point_presentation,
        half_point_text,
        primary_score_presentation,
        primary_value_text,
        standings_value_text,
    },
    components::{
        molecules::{
            game_previews::GamePreview,
            panel::Panel,
            time_row::TimeRow,
            tournament_standings_controls::TournamentStandingsControls,
        },
        organisms::{
            tournament_encounter_summary::{EncounterParticipant, TournamentEncounterSummary},
            tournament_explanations::phase_text,
        },
    },
    functions::games::get::get_game_from_nanoid,
    i18n::*,
    providers::{
        RoundRobinRound,
        RoundRobinState,
        RoundRobinStateStoreFields,
        SwissRound,
        SwissState,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
    },
    responses::{TournamentMemberships, TournamentStandings},
};
use hive_lib::{Color, GameStatus};
use leptos::{portal::Portal, prelude::*};
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{
        elimination::SeriesPlan,
        round_robin::{Config as RoundRobinConfig, PrimaryScore as RoundRobinPrimaryScore},
        standings::Value,
        swiss::{Config as SwissConfig, System as SwissSystem},
        AdjudicatedSideResult,
        FormatConfig,
        GameOutcome,
        Resolution,
        Score,
    },
    tournament_view::{RoundRobinMatchResponse, SlotResponse, SwissByeResponse},
    GameSpeed,
};
use std::collections::HashMap;
use tournamint::{MatchScore, PointSystem};
use uuid::Uuid;

fn player_name(memberships: &TournamentMemberships, player: Uuid) -> String {
    memberships
        .players
        .get(&player)
        .map(|user| user.username.clone())
        .unwrap_or_else(|| String::from("—"))
}

fn format_game_points(score: Score, point_system: PointSystem) -> String {
    standings_value_text(Value::Score(score), game_point_presentation(point_system))
}

fn slot_state(slot: &SlotResponse) -> &'static str {
    // TODO: i18n once copy is approved.
    match slot.resolution {
        Some(Resolution::Result(GameOutcome::Played(_))) => "Played",
        Some(Resolution::Result(GameOutcome::Adjudicated(outcome)))
            if matches!(
                (outcome.white(), outcome.black()),
                (
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit
                )
            ) =>
        {
            "Double forfeit"
        }
        Some(Resolution::Result(GameOutcome::Adjudicated(_))) => "Adjudicated",
        Some(Resolution::Withdrawal(_)) => "Withdrawal",
        Some(Resolution::Clinched) => "Cancelled",
        None => match slot.game.as_ref() {
            Some(game)
                if game.finished
                    || matches!(
                        game.status,
                        GameStatus::Finished(_) | GameStatus::Adjudicated
                    ) =>
            {
                "Finished · result pending"
            }
            Some(game) if game.status == GameStatus::InProgress => "In progress",
            _ if slot.scheduled_at.is_some() => "Scheduled",
            Some(game) if game.status == GameStatus::NotStarted => "Not started",
            Some(_) => "In progress",
            None => "Planned",
        },
    }
}

fn unresolved_slot_state_priority(slot: &SlotResponse) -> u8 {
    if slot.game.as_ref().is_some_and(|game| {
        game.finished
            || matches!(
                game.status,
                GameStatus::Finished(_) | GameStatus::Adjudicated | GameStatus::InProgress
            )
    }) {
        0
    } else if slot.scheduled_at.is_some() {
        1
    } else if slot.game.is_some() {
        2
    } else {
        3
    }
}

fn is_played_slot(slot: &SlotResponse) -> bool {
    matches!(
        slot.resolution,
        Some(Resolution::Result(GameOutcome::Played(_)))
    ) && slot.game_id().is_some()
}

#[component]
fn TournamentGameRow(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
    game_number: usize,
    total_games: usize,
) -> impl IntoView {
    view! {
        {move || {
            let game_slot = game_slot.try_get()?;
            let memberships = common.memberships().get();
            let white_name = player_name(&memberships, game_slot.participants[0]);
            let black_name = player_name(&memberships, game_slot.participants[1]);
            let state = slot_state(&game_slot);
            let score = game_slot
                .awarded_game_points
                .map(|points| {
                    format!(
                        "{} – {}",
                        format_game_points(points[0], point_system),
                        format_game_points(points[1], point_system),
                    )
                });
            let game_id = game_slot.game_id().cloned();
            Some(
                view! {
                    <div class="grid gap-y-1 gap-x-3 py-2 px-2 text-xs grid-cols-[minmax(0,1fr)_auto] dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even odd:bg-odd-light even:bg-even-light">
                        <div class="min-w-0">
                            // TODO: i18n once copy is approved.
                            <div class="font-bold text-gray-700 dark:text-gray-300">
                                {format!("Game {game_number} of {total_games}")}
                            </div>
                            <div class="flex flex-wrap gap-x-1 text-gray-600 dark:text-gray-400">
                                <span class="truncate">{white_name}</span>
                                <span>"–"</span>
                                <span class="truncate">{black_name}</span>
                            </div>
                        </div>
                        <div class="text-right shrink-0">
                            <div class="font-bold tabular-nums">
                                {score.unwrap_or_else(|| String::from("—"))}
                            </div>
                            <div class="text-gray-500">{state}</div>
                            {game_id
                                .map(|game_id| {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <a
                                            class="font-bold hover:underline"
                                            href=format!("/game/{game_id}")
                                        >
                                            "Open game"
                                        </a>
                                    }
                                })}
                        </div>
                    </div>
                },
            )
        }}
    }
}

#[component]
fn LazyTournamentGamePreview(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
    game_number: usize,
    total_games: usize,
) -> impl IntoView {
    let game_id = game_slot
        .get_untracked()
        .game_id()
        .cloned()
        .expect("played Tournament slot has a Game identity");
    let game = Resource::new(move || game_id.clone(), get_game_from_nanoid);
    view! {
        {move || match game.get() {
            Some(Ok(game)) => {
                view! {
                    <div class="flex flex-col items-center min-w-0">
                        // TODO: i18n once copy is approved.
                        <p class="font-bold tracking-wide text-gray-500 uppercase text-[10px]">
                            {format!("Game {game_number}")}
                        </p>
                        <GamePreview game drawer=true />
                    </div>
                }
                    .into_any()
            }
            None | Some(Err(_)) => {
                view! {
                    <TournamentGameRow
                        common
                        game_slot=game_slot.clone()
                        point_system
                        game_number
                        total_games
                    />
                }
                    .into_any()
            }
        }}
    }
}

#[component]
fn TournamentDrawerSlot(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
    game_number: usize,
    total_games: usize,
    played: bool,
) -> impl IntoView {
    move || {
        let Some(game_slot_value) = game_slot.try_get() else {
            return ().into_any();
        };
        if is_played_slot(&game_slot_value) != played {
            return ().into_any();
        }
        if played {
            view! {
                <LazyTournamentGamePreview
                    common
                    game_slot=game_slot.clone()
                    point_system
                    game_number
                    total_games
                />
            }
            .into_any()
        } else {
            view! {
                <TournamentGameRow
                    common
                    game_slot=game_slot.clone()
                    point_system
                    game_number
                    total_games
                />
            }
            .into_any()
        }
    }
}

#[derive(Clone)]
pub struct TournamentDrawerSeries {
    pub(crate) label: Option<&'static str>,
    pub(crate) slots: Vec<ArcField<SlotResponse>>,
    pub(crate) participants: [Uuid; 2],
    pub(crate) score: Option<[u64; 2]>,
    pub(crate) plan: Option<SeriesPlan>,
}

#[component]
pub(crate) fn TournamentGamesDrawer(
    common: Store<TournamentCommon>,
    series: Vec<TournamentDrawerSeries>,
    point_system: PointSystem,
    #[prop(optional_no_strip)] match_points: Option<String>,
    close: Callback<()>,
) -> impl IntoView {
    // Mount both overlay siblings outside transformed bracket/pairing route wrappers.
    view! {
        <Portal>
            <TournamentGamesDrawerBody
                common
                series=series.clone()
                point_system
                match_points=match_points.clone()
                close
            />
        </Portal>
    }
}

#[component]
fn TournamentGamesDrawerBody(
    common: Store<TournamentCommon>,
    series: Vec<TournamentDrawerSeries>,
    point_system: PointSystem,
    match_points: Option<String>,
    close: Callback<()>,
) -> impl IntoView {
    let memberships = common.memberships().get_untracked();
    let participants = series
        .last()
        .map(|series| series.participants)
        .unwrap_or_default();
    let left_name = player_name(&memberships, participants[0]);
    let right_name = player_name(&memberships, participants[1]);
    // TODO: i18n once copy is approved.
    let title = series
        .first()
        .and_then(|series| series.label)
        .unwrap_or("Matchup games");
    view! {
        // TODO: i18n once copy is approved.
        <button
            type="button"
            class="fixed inset-0 z-40 bg-black/35"
            aria-label="Close matchup games"
            on:click=move |_| close.run(())
        ></button>
        <aside
            class="overflow-y-auto overscroll-contain fixed inset-0 z-50 bg-white shadow-2xl md:inset-y-4 md:right-4 md:left-auto md:rounded-xl dark:bg-gray-900 max-h-dvh md:w-[min(34rem,calc(100vw-2rem))]"
            data-testid="round-robin-matchup-drawer"
        >
            <header class="sticky top-0 z-10 py-3 px-4 bg-white border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700">
                <div class="flex gap-3 justify-between items-start">
                    <div class="min-w-0">
                        // TODO: i18n once copy is approved.
                        <p class="font-bold tracking-wide text-gray-500 uppercase text-[10px]">
                            {title}
                        </p>
                        <h2 class="overflow-hidden min-w-0 text-lg font-bold leading-tight wrap-break-word">
                            <span>{left_name}</span>
                            <span class="mx-1.5 font-normal text-gray-500">"vs"</span>
                            <span>{right_name}</span>
                        </h2>
                    </div>
                    // TODO: i18n once copy is approved.
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm shrink-0"
                        on:click=move |_| close.run(())
                    >
                        "Close"
                    </button>
                </div>
            </header>
            <div class="p-3 space-y-5 sm:p-4">
                {match_points
                    .map(|score| {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="font-semibold tabular-nums">
                                {format!("Match points: {score}")}
                            </p>
                        }
                    })}
                {series
                    .into_iter()
                    .map(|series| {
                        view! { <TournamentGamesSeries common series point_system /> }
                    })
                    .collect_view()}
            </div>
        </aside>
    }
}

#[component]
fn TournamentGamesSeries(
    common: Store<TournamentCommon>,
    series: TournamentDrawerSeries,
    point_system: PointSystem,
) -> impl IntoView {
    let left = series.participants[0];
    let score_slots = series.slots.clone();
    let series_score = series.score;
    let matchup_score = move || {
        series_score.map_or_else(
            || {
                matchup_score_text(
                    point_system,
                    &score_slots
                        .iter()
                        .filter_map(|slot| slot.try_get())
                        .collect::<Vec<_>>(),
                    left,
                )
            },
            |score| {
                format!(
                    "{}–{}",
                    half_point_text(score[0]),
                    half_point_text(score[1])
                )
            },
        )
    };
    let memberships = common.memberships().get_untracked();
    let orientation = series.label.map(|_| {
        // TODO: i18n once copy is approved.
        format!(
            "{} vs {}",
            player_name(&memberships, left),
            player_name(&memberships, series.participants[1])
        )
    });
    let strip_slots = series.slots.clone();
    let total_games = series.slots.len();
    let played_game_views = series
        .slots
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, game_slot)| {
            view! {
                <TournamentDrawerSlot
                    common
                    game_slot
                    point_system
                    game_number=index + 1
                    total_games
                    played=true
                />
            }
        })
        .collect_view();
    let compact_game_views = series
        .slots
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, game_slot)| {
            view! {
                <TournamentDrawerSlot
                    common
                    game_slot
                    point_system
                    game_number=index + 1
                    total_games
                    played=false
                />
            }
        })
        .collect_view();
    let played_slots = series.slots.clone();
    let compact_slots = series.slots;
    view! {
        <section class="space-y-3 [&+section]:pt-4 [&+section]:border-t [&+section]:border-gray-200 dark:[&+section]:border-gray-700">
            <div>
                <div class="flex gap-4 justify-between items-center">
                    {series.label.map(|label| view! { <h3 class="font-bold">{label}</h3> })}
                    <span class="text-sm">
                        <RoundRobinResultStrip slots=strip_slots player=left roomy=true />
                    </span> <strong class="ml-auto text-lg tabular-nums">{matchup_score}</strong>
                </div>
                {orientation
                    .map(|orientation| {
                        view! { <p class="text-xs text-gray-500">{orientation}</p> }
                    })}
                {series
                    .plan
                    .map(|plan| {
                        view! {
                            <ol class="mt-2 space-y-1 text-xs text-gray-600 dark:text-gray-300">
                                {plan
                                    .phases
                                    .into_iter()
                                    .map(|phase| {
                                        view! {
                                            <li class="flex flex-wrap gap-x-2 items-center">
                                                <span>
                                                    {phase_text(
                                                        phase.games_per_set,
                                                        phase.set_limit,
                                                        phase.clinch,
                                                    )}
                                                </span>
                                                <TimeRow
                                                    time_control=Some(phase.clock)
                                                    extend_tw_classes="text-xs"
                                                />
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ol>
                        }
                    })}
            </div>
            <section class=move || {
                (!played_slots
                    .iter()
                    .any(|slot| slot.try_get().is_some_and(|slot| is_played_slot(&slot))))
                    .then_some("hidden")
            }>
                // TODO: i18n once copy is approved.
                <h4 class="mb-2 text-xs font-bold tracking-wide text-gray-500 uppercase">
                    "Played games"
                </h4>
                <div class="grid gap-3 items-start grid-cols-[repeat(auto-fit,minmax(min(100%,14rem),1fr))]">
                    {played_game_views}
                </div>
            </section>
            <section class=move || {
                (!compact_slots
                    .iter()
                    .any(|slot| slot.try_get().is_some_and(|slot| !is_played_slot(&slot))))
                    .then_some("hidden")
            }>
                // TODO: i18n once copy is approved.
                <h4 class="mb-2 text-xs font-bold tracking-wide text-gray-500 uppercase">
                    "Other games"
                </h4>
                <div class="overflow-hidden rounded border border-gray-300 dark:border-gray-700">
                    {compact_game_views}
                </div>
            </section>
        </section>
    }
}

pub(crate) fn swiss_round_progress(
    rounds: &[SwissRound],
    round_index: u32,
) -> Option<(usize, usize)> {
    let round = rounds
        .iter()
        .find(|round| round.round_index == round_index)?;
    let total = round.encounters.len() + round.byes.len();
    let resolved = round
        .encounters
        .iter()
        .filter(|encounter| encounter.completion.is_some())
        .count()
        + round.byes.len();
    Some((resolved, total))
}

const CROSS_HEAD: &str =
    "px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:text-gray-300";
const CROSS_CELL: &str = "p-0 text-center";
const CROSS_STICKY_NUMBER_HEAD: &str = "sticky left-0 z-20 w-8 min-w-8 bg-white px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:bg-gray-900 dark:text-gray-300";
const CROSS_STICKY_PLAYER_HEAD: &str = "sticky left-8 z-20 bg-white px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:bg-gray-900 dark:text-gray-300";
const CROSS_STICKY_NUMBER_CELL: &str =
    "sticky left-0 z-10 w-8 min-w-8 bg-white px-1 py-1 text-center font-bold tabular-nums text-gray-500 dark:bg-gray-900 dark:text-gray-400";
const CROSS_STICKY_PLAYER_CELL: &str =
    "sticky left-8 z-10 max-w-40 bg-white px-1 py-1 text-left whitespace-nowrap dark:bg-gray-900";

#[derive(Clone)]
struct RoundRobinCell {
    slots: Vec<ArcField<SlotResponse>>,
    match_result: Option<RoundRobinMatchResponse>,
}

#[derive(Clone)]
struct RoundRobinTable {
    order: Vec<(Option<u32>, Uuid, String)>,
    cells: HashMap<(Uuid, Uuid), RoundRobinCell>,
}

fn unordered(left: Uuid, right: Uuid) -> (Uuid, Uuid) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn round_robin_cells(
    rounds: &[RoundRobinRound],
    round_robin: Store<RoundRobinState>,
) -> HashMap<(Uuid, Uuid), RoundRobinCell> {
    let mut cells = HashMap::new();
    for round in rounds {
        for slot in &round.slots {
            let slot_field: ArcField<SlotResponse> =
                round_robin.slots().at_key(slot.slot_id).into();
            let participants = slot_field.get_untracked().participants;
            let key = unordered(participants[0], participants[1]);
            cells
                .entry(key)
                .or_insert_with(|| RoundRobinCell {
                    slots: Vec::new(),
                    match_result: None,
                })
                .slots
                .push(slot_field);
        }
    }
    for encounter in round_robin.matches().get() {
        if let Some(cell) = cells.get_mut(&unordered(
            encounter.participants[0],
            encounter.participants[1],
        )) {
            cell.match_result = Some(encounter);
        }
    }
    cells
}

fn ranked(
    memberships: &TournamentMemberships,
    standings: &TournamentStandings,
    configuration: &RoundRobinConfig,
) -> Vec<(Option<u32>, Uuid, String)> {
    if let Some(standings) = &standings.snapshot {
        return display_standing_rows(standings, &memberships.withdrawn)
            .into_iter()
            .map(|standing| {
                (
                    standing.rank,
                    standing.row.user_id,
                    primary_value_text(
                        standing.row.primary_score,
                        &FormatConfig::RoundRobin(configuration.clone()),
                    ),
                )
            })
            .collect();
    }
    let mut players = memberships
        .players
        .values()
        .map(|player| (None, player.uid, String::from("—")))
        .collect::<Vec<_>>();
    players.sort_by(|left, right| {
        memberships.players[&left.1]
            .username
            .cmp(&memberships.players[&right.1].username)
    });
    players
}

fn round_robin_matrix_order(
    memberships: &TournamentMemberships,
    standings: &TournamentStandings,
    configuration: &RoundRobinConfig,
    viewer: Option<Uuid>,
) -> Vec<(Option<u32>, Uuid, String)> {
    let mut players = ranked(memberships, standings, configuration);
    if let Some(position) =
        viewer.and_then(|viewer| players.iter().position(|(_, player, _)| *player == viewer))
    {
        let viewer = players.remove(position);
        players.insert(0, viewer);
    }
    players
}

fn result_marker(slot: &SlotResponse, player: Uuid) -> &'static str {
    let Some(resolution) = slot.resolution else {
        return if slot.game.is_some() { "*" } else { "–" };
    };
    let outcome = match resolution {
        Resolution::Result(outcome) | Resolution::Withdrawal(outcome) => outcome,
        Resolution::Clinched => return "–",
    };
    match if slot.participants[0] == player {
        outcome.white().score()
    } else {
        outcome.black().score()
    } {
        MatchScore::Win => "W",
        MatchScore::Draw => "D",
        MatchScore::Loss => "L",
    }
}

fn result_marker_color(marker: &str) -> &'static str {
    match marker {
        "W" => "text-grasshopper-green",
        "D" => "text-pillbug-teal",
        "L" => "text-ladybug-red",
        _ => "text-gray-500 dark:text-gray-400",
    }
}

#[component]
fn RoundRobinResultStrip(
    slots: Vec<ArcField<SlotResponse>>,
    player: Uuid,
    #[prop(default = false)] roomy: bool,
) -> impl IntoView {
    let marker_gap = if roomy { "gap-1" } else { "gap-px" };
    view! {
        <span class=format!(
            "inline-flex items-center justify-center font-mono font-bold leading-none {marker_gap}",
        )>
            {slots
                .into_iter()
                .map(|slot| {
                    view! {
                        {move || {
                            let slot = slot.try_get()?;
                            let marker = result_marker(&slot, player);
                            let color = result_marker_color(marker);
                            Some(
                                view! {
                                    <span class=format!(
                                        "inline-flex w-[1ch] shrink-0 justify-center {color}",
                                    )>{marker}</span>
                                },
                            )
                        }}
                    }
                })
                .collect_view()}
        </span>
    }
}

#[component]
fn RoundRobinMatrixCell(
    slots: Vec<ArcField<SlotResponse>>,
    point_system: PointSystem,
    primary_match_points: bool,
    row: Uuid,
    column: Uuid,
    pair: (Uuid, Uuid),
    opened: RwSignal<Option<(Uuid, Uuid)>>,
    hovered: RwSignal<Option<(Uuid, Uuid)>>,
    match_result: Option<RoundRobinMatchResponse>,
) -> impl IntoView {
    let strip = slots.clone();
    let score = move || {
        let slots = slots
            .iter()
            .filter_map(|slot| slot.try_get())
            .collect::<Vec<_>>();
        if primary_match_points {
            round_robin_match_score_text(match_result.as_ref(), row)
        } else {
            matchup_score_text(point_system, &slots, row)
        }
    };
    view! {
        <td class=move || {
            let on_axis = opened
                .get()
                .is_some_and(|(selected_row, selected_column)| {
                    selected_row == row || selected_column == column
                })
                || hovered
                    .get()
                    .is_some_and(|(selected_row, selected_column)| {
                        selected_row == row || selected_column == column
                    });
            if on_axis {
                format!("{CROSS_CELL} bg-pillbug-teal/10")
            } else {
                CROSS_CELL.to_string()
            }
        }>
            <button
                type="button"
                class=move || {
                    if opened.get().is_some_and(|opened| unordered(opened.0, opened.1) == pair) {
                        "flex h-10 w-full min-w-10 flex-col items-center justify-center bg-pillbug-teal px-0.5 font-bold tabular-nums text-white ring-2 ring-inset ring-pillbug-teal/30"
                    } else {
                        "flex h-10 w-full min-w-10 flex-col items-center justify-center px-0.5 tabular-nums hover:bg-pillbug-teal/20"
                    }
                }
                on:mouseenter=move |_| hovered.set(Some((row, column)))
                on:click=move |_| opened.set(Some((row, column)))
            >
                <span class="text-[9px]">
                    <RoundRobinResultStrip slots=strip player=row />
                </span>
                <strong class="mt-0.5 leading-none text-[10px]">{score}</strong>
            </button>
        </td>
    }
}

fn round_robin_match_score_text(
    encounter: Option<&RoundRobinMatchResponse>,
    player: Uuid,
) -> String {
    encounter
        .and_then(|encounter| {
            encounter.completion.map(|completed| {
                let side = usize::from(encounter.participants[1] == player);
                format!(
                    "{}–{}",
                    completed.match_points[side].value(),
                    completed.match_points[1 - side].value()
                )
            })
        })
        .unwrap_or_else(|| String::from("—"))
}

fn matchup_score_text(point_system: PointSystem, slots: &[SlotResponse], player: Uuid) -> String {
    let mut points = [0u32; 2];
    let mut has_award = false;
    for slot in slots {
        let Some(award) = slot.awarded_game_points else {
            continue;
        };
        let side = usize::from(slot.participants[1] == player);
        points[0] = points[0].saturating_add(award[side].value());
        points[1] = points[1].saturating_add(award[1 - side].value());
        has_award = true;
    }
    if !has_award {
        return String::from("—");
    }
    let presentation = game_point_presentation(point_system);
    let value = format!(
        "{}–{}",
        standings_value_text(Value::Score(Score::new(points[0])), presentation),
        standings_value_text(Value::Score(Score::new(points[1])), presentation),
    );
    value
}

#[component]
fn RoundRobinMatrixPlayerRow(
    common: Store<TournamentCommon>,
    point_system: PointSystem,
    primary_match_points: bool,
    row_index: usize,
    row: Uuid,
    total: String,
    columns: Vec<(Uuid, Option<RoundRobinCell>)>,
    opened: RwSignal<Option<(Uuid, Uuid)>>,
    hovered: RwSignal<Option<(Uuid, Uuid)>>,
) -> impl IntoView {
    let cells = columns
        .into_iter()
        .map(|(column, cell)| {
            if row == column {
                return view! {
                    <td class=format!(
                        "{CROSS_CELL} bg-gray-100 text-gray-400 dark:bg-gray-800 dark:text-gray-600",
                    )>"—"</td>
                }
                .into_any();
            }
            let pair = unordered(row, column);
            let Some(cell) = cell else {
                return view! {
                    <td class=CROSS_CELL>
                        <span class="text-gray-400 dark:text-gray-600">"·"</span>
                    </td>
                }
                .into_any();
            };
            view! {
                <RoundRobinMatrixCell
                    slots=cell.slots
                    match_result=cell.match_result
                    primary_match_points
                    point_system
                    row
                    column
                    pair
                    opened
                    hovered
                />
            }
            .into_any()
        })
        .collect_view();
    view! {
        <tr class=move || {
            if opened.get().is_some_and(|(selected_row, _)| selected_row == row)
                || hovered.get().is_some_and(|(selected_row, _)| selected_row == row)
            {
                "ui-dense-table-row !bg-pillbug-teal/25"
            } else {
                "ui-dense-table-row"
            }
        }>
            <td class=CROSS_STICKY_NUMBER_CELL>{row_index + 1}</td>
            <td class=CROSS_STICKY_PLAYER_CELL>
                {move || {
                    let memberships = common.memberships().get();
                    let name = player_name(&memberships, row);
                    let name_title = name.clone();
                    let withdrawn = memberships.withdrawn.contains(&row);
                    view! {
                        <span
                            class=if withdrawn {
                                "block max-w-40 truncate line-through"
                            } else {
                                "block max-w-40 truncate"
                            }
                            title=name_title
                        >
                            {name}
                        </span>
                    }
                }}
            </td>
            {cells}
            <td class=CROSS_CELL>
                <strong>{total}</strong>
            </td>
        </tr>
    }
}

#[component]
fn RoundRobinPlayerMatchup(
    common: Store<TournamentCommon>,
    point_system: PointSystem,
    primary_match_points: bool,
    player: Uuid,
    opponent: Uuid,
    opponent_number: usize,
    slots: Vec<ArcField<SlotResponse>>,
    match_result: Option<RoundRobinMatchResponse>,
) -> impl IntoView {
    let opponent_name = player_name(&common.memberships().get_untracked(), opponent);
    let score_slots = slots.clone();
    let score = move || {
        if primary_match_points {
            return round_robin_match_score_text(match_result.as_ref(), player);
        }
        matchup_score_text(
            point_system,
            &score_slots
                .iter()
                .filter_map(|slot| slot.try_get())
                .collect::<Vec<_>>(),
            player,
        )
    };
    let progress_slots = slots.clone();
    let resolved = move || {
        progress_slots
            .iter()
            .filter(|slot| slot.try_get().is_some_and(|slot| slot.resolution.is_some()))
            .count()
    };
    let total = slots.len();
    let rows = slots
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            view! {
                <TournamentGameRow
                    common
                    game_slot=slot
                    point_system
                    game_number=index + 1
                    total_games=total
                />
            }
        })
        .collect_view();

    view! {
        <details class="overflow-hidden rounded border border-gray-300 dark:border-gray-700">
            <summary class="grid gap-y-1 gap-x-3 items-center py-2 px-3 cursor-pointer grid-cols-[minmax(0,1fr)_auto] marker:text-gray-400">
                <span class="min-w-0 text-sm font-bold truncate">
                    <span class="mr-2 font-mono text-xs text-gray-500 dark:text-gray-400">
                        {format!("#{opponent_number}")}
                    </span>
                    {opponent_name}
                </span>
                <strong class="text-sm tabular-nums">{score}</strong>
                <span class="text-xs">
                    <RoundRobinResultStrip slots=slots.clone() player roomy=true />
                </span>
                {move || {
                    (resolved() < total)
                        .then(|| {
                            view! {
                                // TODO: i18n once copy is approved.
                                <span class="tabular-nums text-right text-gray-500 dark:text-gray-400 text-[10px]">
                                    {format!("{}/{total} games", resolved())}
                                </span>
                            }
                        })
                }}
            </summary>
            <div class="border-t border-gray-200 dark:border-gray-700">{rows}</div>
        </details>
    }
}

#[component]
fn RoundRobinPlayerLedger(
    common: Store<TournamentCommon>,
    point_system: PointSystem,
    primary_match_points: bool,
    player: Uuid,
    table: Memo<RoundRobinTable>,
) -> impl IntoView {
    view! {
        <div class="space-y-3">
            {move || {
                let table = table.get();
                let order = &table.order;
                let cells = &table.cells;
                let Some((rank, _, total_score)) = order
                    .iter()
                    .find(|(_, candidate, _)| *candidate == player)
                    .cloned() else {
                    return view! {
                        // TODO: i18n once copy is approved.
                        <p class="ui-empty-state">"Choose a player to see their results."</p>
                    }
                        .into_any()
                };
                let player_name = player_name(&common.memberships().get(), player);
                let withdrawn = common.memberships().get().withdrawn.contains(&player);
                let mut resolved_games = 0;
                let mut total_games = 0;
                let matchups = order
                    .iter()
                    .enumerate()
                    .filter_map(|(index, (_, opponent, _))| {
                        if *opponent == player {
                            return None;
                        }
                        let cell = cells.get(&unordered(player, *opponent))?;
                        resolved_games
                            += cell
                                .slots
                                .iter()
                                .filter(|slot| {
                                    slot.try_get().is_some_and(|slot| slot.resolution.is_some())
                                })
                                .count();
                        total_games += cell.slots.len();
                        Some((index + 1, *opponent, cell.slots.clone(), cell.match_result.clone()))
                    })
                    .collect::<Vec<_>>();
                let matchup_views = matchups
                    .into_iter()
                    .map(|(opponent_number, opponent, slots, match_result)| {
                        view! {
                            <RoundRobinPlayerMatchup
                                common
                                primary_match_points
                                match_result
                                point_system
                                player
                                opponent
                                opponent_number
                                slots
                            />
                        }
                    })
                    .collect_view();
                view! {
                    <div class="flex gap-3 justify-between items-end px-1">
                        <div class="min-w-0">
                            <p class=if withdrawn {
                                "block max-w-full font-bold line-through truncate"
                            } else {
                                "block max-w-full font-bold truncate"
                            }>{player_name}</p>
                            // TODO: i18n once copy is approved.
                            <p class="text-xs tabular-nums text-gray-500 dark:text-gray-400">
                                {format!(
                                    "Rank {} · {total_score} points",
                                    rank
                                        .map(|rank| rank.to_string())
                                        .unwrap_or_else(|| String::from("—")),
                                )}
                            </p>
                        </div>
                        // TODO: i18n once copy is approved.
                        <p class="text-xs tabular-nums text-right text-gray-500 dark:text-gray-400 shrink-0">
                            {format!("{resolved_games}/{total_games} games")}
                        </p>
                    </div>
                    {if matchup_views.is_empty() {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="text-sm text-gray-500">"No opponent results yet."</p>
                        }
                            .into_any()
                    } else {
                        view! { <div class="space-y-2">{matchup_views}</div> }.into_any()
                    }}
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
pub fn RoundRobinCrosstable(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    user_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let point_system = round_robin
        .configuration()
        .get_untracked()
        .game_point_system;
    let primary_match_points = round_robin.configuration().get_untracked().primary_score
        == RoundRobinPrimaryScore::MatchPoints;
    let i18n = use_i18n();
    let hovered = RwSignal::new(None::<(Uuid, Uuid)>);
    let opened = RwSignal::new(None::<(Uuid, Uuid)>);
    let close_drawer = Callback::new(move |()| opened.set(None));
    let mobile_choice = RwSignal::new(None::<Uuid>);
    let table = Memo::new_with_compare(
        move |_| {
            let viewer = user_id.get();
            let memberships = common.memberships().get();
            let standings = common.standings().get();
            let configuration = round_robin.configuration().get();
            let rounds = round_robin.rounds().get();
            RoundRobinTable {
                order: round_robin_matrix_order(&memberships, &standings, &configuration, viewer),
                cells: round_robin_cells(&rounds, round_robin),
            }
        },
        |_, _| true,
    );
    let mobile_player = Signal::derive(move || {
        table.with(|table| {
            mobile_choice
                .get()
                .filter(|chosen| table.order.iter().any(|(_, player, _)| player == chosen))
                .or_else(|| table.order.first().map(|(_, player, _)| *player))
        })
    });
    let opened_cell = Signal::derive(move || {
        let (left, right) = opened.get()?;
        table.with(|table| table.cells.get(&unordered(left, right)).cloned())
    });

    view! {
        <Panel title=move || common.lifecycle().get().name class="min-w-0">
            <div class="space-y-3 md:hidden" data-testid="round-robin-player-results">
                <div>
                    // TODO: i18n once copy is approved.
                    <label class="block mb-1 font-bold tracking-wide text-gray-500 uppercase text-[10px]">
                        "Player results"
                    </label>
                    <select
                        class="w-full ui-field-select"
                        prop:value=move || {
                            mobile_player.get().map(|player| player.to_string()).unwrap_or_default()
                        }
                        on:change=move |event| {
                            mobile_choice.set(Uuid::parse_str(&event_target_value(&event)).ok())
                        }
                    >
                        {move || {
                            table
                                .with(|table| table.order.clone())
                                .into_iter()
                                .enumerate()
                                .map(|(index, (_, player, _))| {
                                    let name = player_name(&common.memberships().get(), player);
                                    let withdrawn = common
                                        .memberships()
                                        .get()
                                        .withdrawn
                                        .contains(&player);
                                    view! {
                                        <option
                                            class=withdrawn.then_some("line-through")
                                            value=player.to_string()
                                        >
                                            {format!("#{} {name}", index + 1)}
                                        </option>
                                    }
                                })
                                .collect_view()
                        }}
                    </select>
                </div>
                {move || {
                    mobile_player
                        .get()
                        .map(|player| {
                            view! {
                                <RoundRobinPlayerLedger
                                    common
                                    point_system
                                    primary_match_points
                                    player
                                    table
                                />
                            }
                        })
                }}
            </div>
            <div class="hidden space-y-3 md:block">
                <div class="overflow-x-auto" data-testid="round-robin-crosstable">
                    <table class="min-w-full text-xs border-collapse">
                        <thead>
                            <tr>
                                <th class=CROSS_STICKY_NUMBER_HEAD>"#"</th>
                                <th class=CROSS_STICKY_PLAYER_HEAD>
                                    {t!(i18n, tournaments.view.common.player)}
                                </th>
                                {move || {
                                    table
                                        .with(|table| table.order.clone())
                                        .into_iter()
                                        .enumerate()
                                        .map(|(index, (_, player, _))| {
                                            let name = player_name(&common.memberships().get(), player);
                                            view! {
                                                <th class=move || {
                                                    let selected = opened
                                                        .get()
                                                        .is_some_and(|(_, column)| column == player)
                                                        || hovered
                                                            .get()
                                                            .is_some_and(|(_, column)| column == player);
                                                    if selected {
                                                        format!("{CROSS_HEAD} bg-pillbug-teal/20")
                                                    } else {
                                                        CROSS_HEAD.to_string()
                                                    }
                                                }>
                                                    <span
                                                        class="inline-block py-1 w-10 font-bold tabular-nums"
                                                        title=name
                                                    >
                                                        {index + 1}
                                                    </span>
                                                </th>
                                            }
                                        })
                                        .collect_view()
                                }}
                                <th class=CROSS_HEAD>
                                    {t!(i18n, tournaments.view.encounters.total)}
                                </th>
                            </tr>
                        </thead>
                        <tbody on:mouseleave=move |_| {
                            hovered.set(None)
                        }>
                            {move || {
                                let table = table.get();
                                let order = &table.order;
                                let cells = &table.cells;
                                order
                                    .iter()
                                    .enumerate()
                                    .map(|(row_index, (_, row, total))| {
                                        let (row, total) = (*row, total.clone());
                                        let columns = order
                                            .iter()
                                            .map(|(_, column, _)| {
                                                let column = *column;
                                                (column, cells.get(&unordered(row, column)).cloned())
                                            })
                                            .collect();
                                        view! {
                                            <RoundRobinMatrixPlayerRow
                                                common
                                                primary_match_points
                                                point_system
                                                row_index
                                                row
                                                total
                                                columns
                                                opened
                                                hovered
                                            />
                                        }
                                    })
                                    .collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
            </div>
            {move || {
                let (left, right) = opened.get()?;
                let cell = opened_cell.get()?;
                let match_points = cell
                    .match_result
                    .as_ref()
                    .map(|encounter| round_robin_match_score_text(Some(encounter), left));
                Some(
                    view! {
                        <TournamentGamesDrawer
                            common
                            match_points=match_points
                            series=vec![
                                TournamentDrawerSeries {
                                    label: None,
                                    slots: cell.slots,
                                    participants: [left, right],
                                    score: None,
                                    plan: None,
                                },
                            ]
                            point_system
                            close=close_drawer
                        />
                    },
                )
            }}
        </Panel>
    }
}

const ROUND_BROWSER_PAGE_SIZE: usize = 10;

#[derive(Clone)]
enum SwissRoundItem {
    Encounter(SwissEncounterView),
    Bye(SwissByeResponse),
}

#[derive(Clone)]
struct SwissEncounterView {
    pairing_index: u32,
    participants: [Uuid; 2],
    pre_round_primary_scores: [Score; 2],
    rating_snapshots: [Option<u32>; 2],
    slots: Vec<ArcField<SlotResponse>>,
}

fn is_double_swiss(configuration: &SwissConfig) -> bool {
    matches!(&configuration.system, SwissSystem::DoubleSwiss(_))
}

fn swiss_primary_score_text(configuration: &SwissConfig, score: Score) -> String {
    standings_value_text(
        Value::Score(score),
        primary_score_presentation(&FormatConfig::Swiss(configuration.clone())),
    )
}

fn swiss_rating(
    configuration: &SwissConfig,
    memberships: &TournamentMemberships,
    player: Uuid,
    snapshot: Option<u32>,
) -> Option<u64> {
    snapshot.map(u64::from).or_else(|| {
        let speed = GameSpeed::from(configuration.clock);
        memberships
            .players
            .get(&player)
            .map(|user| user.rating_for_speed(&speed))
    })
}

fn swiss_encounter_rating(
    configuration: &SwissConfig,
    memberships: &TournamentMemberships,
    encounter: &SwissEncounterView,
    player: Uuid,
) -> Option<u64> {
    let side = encounter
        .participants
        .iter()
        .position(|participant| *participant == player)?;
    let snapshot = encounter.rating_snapshots[side].or_else(|| {
        encounter.slots.iter().find_map(|slot| {
            let slot = slot.try_get()?;
            slot.resolution?;
            let side = slot
                .participants
                .iter()
                .position(|participant| *participant == player)?;
            slot.game.as_ref()?.ratings[side]
        })
    });
    swiss_rating(configuration, memberships, player, snapshot)
}

fn swiss_encounter_result(
    point_system: PointSystem,
    encounter: &SwissEncounterView,
    left: Uuid,
) -> String {
    let mut totals = [0u32; 2];
    let mut scored = 0usize;
    let slots = encounter
        .slots
        .iter()
        .filter_map(|slot| slot.try_get())
        .collect::<Vec<_>>();
    let all_present = slots.len() == encounter.slots.len();
    let mut all_scored = all_present;
    for slot in &slots {
        let Some(points) = slot.awarded_game_points else {
            all_scored = false;
            continue;
        };
        let oriented = if slot.participants[0] == left {
            points
        } else {
            [points[1], points[0]]
        };
        let Some(left_total) = totals[0].checked_add(oriented[0].value()) else {
            return String::from("—");
        };
        let Some(right_total) = totals[1].checked_add(oriented[1].value()) else {
            return String::from("—");
        };
        totals = [left_total, right_total];
        scored += 1;
    }
    let resolved = all_present && slots.iter().all(|slot| slot.resolution.is_some());
    let unresolved_state = slots
        .iter()
        .filter(|slot| slot.resolution.is_none())
        .min_by_key(|slot| unresolved_slot_state_priority(slot))
        .map(|slot| slot_state(slot));
    // TODO: i18n once copy is approved.
    let state = unresolved_state.unwrap_or(if all_scored && scored != 0 {
        ""
    } else if resolved {
        "Resolved"
    } else {
        "Pending"
    });
    if scored == 0 {
        return state.to_string();
    }
    let score = format!(
        "{} – {}",
        format_game_points(Score::new(totals[0]), point_system),
        format_game_points(Score::new(totals[1]), point_system),
    );
    if state.is_empty() {
        score
    } else {
        format!("{score} · {state}")
    }
}

fn swiss_round_items(round: &SwissRound, swiss: Store<SwissState>) -> Vec<SwissRoundItem> {
    round
        .encounters
        .iter()
        .map(|encounter| {
            SwissRoundItem::Encounter(SwissEncounterView {
                pairing_index: encounter.pairing_index,
                participants: encounter.participants,
                pre_round_primary_scores: encounter.pre_round_primary_scores,
                rating_snapshots: encounter.rating_snapshots,
                slots: encounter
                    .slot_ids
                    .iter()
                    .map(|slot_id| swiss.slots().at_key(*slot_id).into())
                    .collect(),
            })
        })
        .chain(round.byes.iter().copied().map(SwissRoundItem::Bye))
        .collect()
}

fn swiss_round_item_matches(
    memberships: &TournamentMemberships,
    item: &SwissRoundItem,
    needle: &str,
) -> bool {
    if needle.is_empty() {
        return true;
    }
    match item {
        SwissRoundItem::Encounter(encounter) => encounter.participants.iter().any(|player| {
            player_name(memberships, *player)
                .to_lowercase()
                .contains(needle)
        }),
        SwissRoundItem::Bye(bye_) => player_name(memberships, bye_.player)
            .to_lowercase()
            .contains(needle),
    }
}

#[component]
fn SwissRoundLedgerItem(
    common: Store<TournamentCommon>,
    configuration: SwissConfig,
    item: SwissRoundItem,
    round_index: u32,
    open_pairing: Callback<(u32, u32)>,
    interactive: bool,
) -> impl IntoView {
    match item {
        SwissRoundItem::Encounter(encounter) => {
            let pairing_index = encounter.pairing_index;
            let participants = encounter.participants;
            let memberships = common.memberships().get_untracked();
            let left_name = player_name(&memberships, participants[0]);
            let right_name = player_name(&memberships, participants[1]);
            view! {
                {move || {
                    let first_slot = encounter.slots.first().and_then(|slot| slot.try_get());
                    let game_id = first_slot.as_ref().and_then(|slot| slot.game_id().cloned());
                    let color = |player| {
                        first_slot
                            .as_ref()
                            .and_then(|slot| {
                                slot.participants
                                    .iter()
                                    .position(|participant| *participant == player)
                                    .map(|side| if side == 0 { Color::White } else { Color::Black })
                            })
                    };
                    let detail = |side: usize| {
                        let rating = swiss_encounter_rating(
                                &configuration,
                                &memberships,
                                &encounter,
                                participants[side],
                            )
                            .map_or_else(|| String::from("—"), |rating| rating.to_string());
                        let score = swiss_primary_score_text(
                            &configuration,
                            encounter.pre_round_primary_scores[side],
                        );
                        format!("{rating} · {score} pts")
                    };
                    let left = EncounterParticipant {
                        name: left_name.clone(),
                        detail: Some(detail(0)),
                        color: color(participants[0]),
                    };
                    let right = EncounterParticipant {
                        name: right_name.clone(),
                        detail: Some(detail(1)),
                        color: color(participants[1]),
                    };
                    let result = swiss_encounter_result(
                        configuration.game_point_system,
                        &encounter,
                        first_slot.as_ref().map_or(participants[0], SlotResponse::white),
                    );
                    if interactive {
                        // TODO: i18n once copy is approved.
                        view! {
                            <article class="ui-dense-table-row">
                                <button
                                    type="button"
                                    class="block py-2 px-3 w-full text-left hover:bg-pillbug-teal/10"
                                    on:click=move |_| open_pairing.run((round_index, pairing_index))
                                >
                                    <TournamentEncounterSummary
                                        identity=format!("#{}", pairing_index + 1)
                                        left
                                        right=Some(right)
                                        result
                                        openable=true
                                    />
                                </button>
                            </article>
                        }
                            .into_any()
                    } else if let Some(game_id) = game_id {
                        view! {
                            <article class="ui-dense-table-row">
                                <a
                                    class="block py-2 px-3 w-full text-left no-link-style hover:bg-pillbug-teal/10"
                                    href=format!("/game/{game_id}")
                                >
                                    <TournamentEncounterSummary
                                        identity=format!("#{}", pairing_index + 1)
                                        left
                                        right=Some(right)
                                        result
                                        openable=true
                                    />
                                </a>
                            </article>
                        }
                            .into_any()
                    } else {
                        view! {
                            <article class="py-2 px-3 ui-dense-table-row">
                                <TournamentEncounterSummary
                                    identity=format!("#{}", pairing_index + 1)
                                    left
                                    right=Some(right)
                                    result
                                />
                            </article>
                        }
                            .into_any()
                    }
                }}
            }
            .into_any()
        }
        SwissRoundItem::Bye(bye_) => {
            let memberships = common.memberships().get_untracked();
            let (name, rating, score) = {
                (
                    player_name(&memberships, bye_.player),
                    swiss_rating(
                        &configuration,
                        &memberships,
                        bye_.player,
                        bye_.rating_snapshot,
                    ),
                    swiss_primary_score_text(&configuration, bye_.pre_round_primary_score),
                )
            };
            // TODO: i18n once copy is approved.
            let state = String::from("Bye");
            // TODO: i18n once copy is approved.
            let participant = EncounterParticipant {
                name,
                color: None,
                detail: Some(format!(
                    "{} · {} pts",
                    rating.map_or_else(|| String::from("—"), |rating| rating.to_string()),
                    score,
                )),
            };
            view! {
                <article class="py-2 px-3 ui-dense-table-row">
                    <TournamentEncounterSummary
                        identity=String::from("—")
                        left=participant
                        right=None
                        result=state
                    />
                </article>
            }
            .into_any()
        }
    }
}

#[component]
pub fn SwissRoundBrowser(
    common: Store<TournamentCommon>,
    swiss: Store<SwissState>,
) -> impl IntoView {
    let selected = RwSignal::new(swiss.rounds().get_untracked().len().saturating_sub(1));
    let search = RwSignal::new(String::new());
    let page = RwSignal::new(1usize);
    let opened_pairing = RwSignal::new(None::<(u32, u32)>);
    let open_pairing = Callback::new(move |pairing| opened_pairing.set(Some(pairing)));
    let close_drawer = Callback::new(move |()| opened_pairing.set(None));
    let suggestions_id = StoredValue::new(format!(
        "round-browser-suggestions-{}",
        common.lifecycle().get_untracked().tournament_id.0
    ));
    let entrant_names = Signal::derive(move || {
        common
            .memberships()
            .get()
            .players
            .values()
            .map(|player| player.username.clone())
            .collect::<Vec<_>>()
    });
    let filtered_items = Memo::new_with_compare(
        move |_| {
            let needle = search.get().trim().to_lowercase();
            let memberships = common.memberships().get();
            swiss
                .rounds()
                .get()
                .get(selected.get())
                .map(|round| swiss_round_items(round, swiss))
                .unwrap_or_default()
                .into_iter()
                .filter(|item| swiss_round_item_matches(&memberships, item, &needle))
                .collect::<Vec<_>>()
        },
        |_, _| true,
    );
    let total = Signal::derive(move || filtered_items.with(Vec::len));
    let on_page_change = Callback::new(move |next| page.set(next));
    let on_query_change = Callback::new(move |query| {
        search.set(query);
        page.set(1);
    });
    Effect::new(move |_| {
        let last = swiss.rounds().get().len().saturating_sub(1);
        if selected.get_untracked() > last {
            selected.set(last);
            page.set(1);
        }
    });
    Effect::new(move |_| {
        let last = total.get().div_ceil(ROUND_BROWSER_PAGE_SIZE).max(1);
        if page.get_untracked() > last {
            page.set(last);
        } else if page.get_untracked() == 0 {
            page.set(1);
        }
    });

    view! {
        <Panel
            // TODO: i18n once copy is approved.
            title=move || common.lifecycle().get().name
            class="mx-auto min-w-0 max-w-6xl"
            body_class="space-y-3"
        >
            {move || {
                {
                    let configuration = swiss.configuration().get();
                    let rounds = swiss.rounds().get();
                    let selected_index = selected.get().min(rounds.len().saturating_sub(1));
                    let round = rounds.get(selected_index)?.clone();
                    let (resolved, encounter_count) = swiss_round_progress(
                            &rounds,
                            round.round_index,
                        )
                        .unwrap_or((0, round.encounters.len() + round.byes.len()));
                    let configured_rounds = configuration
                        .rounds
                        .resolved_rounds()
                        .map(|rounds| rounds.get() as usize)
                        .unwrap_or(rounds.len());
                    let previous = selected_index.checked_sub(1);
                    let next = (selected_index + 1 < rounds.len()).then_some(selected_index + 1);
                    Some(
                        view! {
                            <div class="space-y-3">
                                <div class="flex flex-wrap gap-y-1 gap-x-4 justify-between items-baseline">
                                    // TODO: i18n once copy is approved.
                                    <strong class="text-base tabular-nums">
                                        {format!(
                                            "Round {} of {configured_rounds}",
                                            round.round_index + 1,
                                        )}
                                    </strong>
                                    // TODO: i18n once copy is approved.
                                    <span class="text-xs font-semibold tabular-nums text-gray-500">
                                        {format!("{resolved} of {encounter_count} resolved")}
                                    </span>
                                </div>
                                <div class="flex gap-2 justify-between">
                                    // TODO: i18n once copy is approved.
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        disabled=previous.is_none()
                                        on:click=move |_| {
                                            if let Some(previous) = previous {
                                                selected.set(previous);
                                                page.set(1);
                                            }
                                        }
                                    >
                                        "Previous round"
                                    </button>
                                    // TODO: i18n once copy is approved.
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        disabled=next.is_none()
                                        on:click=move |_| {
                                            if let Some(next) = next {
                                                selected.set(next);
                                                page.set(1);
                                            }
                                        }
                                    >
                                        "Next round"
                                    </button>
                                </div>
                                <div class="flex items-center py-2 px-2 min-w-0 sm:px-3 border-y border-black/10 dark:border-white/10">
                                    <TournamentStandingsControls
                                        page=page.into()
                                        total
                                        page_size=ROUND_BROWSER_PAGE_SIZE
                                        query=search.into()
                                        on_query_change
                                        on_page_change
                                        entrant_names
                                        suggestions_id=suggestions_id.get_value()
                                    />
                                </div>
                            </div>
                        },
                    )
                }
            }}
            <div class="overflow-hidden rounded border border-gray-200 dark:border-gray-700">
                <div class="hidden gap-2 py-2 px-3 text-xs font-bold tracking-wide text-gray-500 uppercase md:grid md:grid-cols-[3rem_minmax(0,1fr)_8rem_minmax(0,1fr)]">
                    // TODO: i18n once copy is approved.
                    <span class="text-center">"Pair"</span>
                    // TODO: i18n once copy is approved.
                    <span>"Player · rating · pre-round score"</span>
                    // TODO: i18n once copy is approved.
                    <span class="text-center">"Result / state"</span>
                    // TODO: i18n once copy is approved.
                    <span>"Player · rating · pre-round score"</span>
                </div>
                {move || {
                    let start = page
                        .get()
                        .saturating_sub(1)
                        .saturating_mul(ROUND_BROWSER_PAGE_SIZE);
                    let visible = filtered_items
                        .get()
                        .into_iter()
                        .skip(start)
                        .take(ROUND_BROWSER_PAGE_SIZE)
                        .collect::<Vec<_>>();
                    if visible.is_empty() {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="ui-empty-state">"No pairings match this search."</p>
                        }
                            .into_any()
                    } else {
                        let configuration = swiss.configuration().get();
                        let interactive = is_double_swiss(&configuration);
                        let round_index = swiss
                            .rounds()
                            .get()
                            .get(selected.get())
                            .map(|round| round.round_index)
                            .unwrap_or_default();
                        visible
                            .into_iter()
                            .map(|item| {
                                view! {
                                    <SwissRoundLedgerItem
                                        common
                                        configuration=configuration.clone()
                                        item
                                        round_index
                                        open_pairing
                                        interactive
                                    />
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                }}
            </div>
        </Panel>
        {move || {
            let (round_index, pairing_index) = opened_pairing.get()?;
            let encounter = swiss
                .rounds()
                .get()
                .iter()
                .find(|round| round.round_index == round_index)?
                .encounters
                .iter()
                .find(|encounter| encounter.pairing_index == pairing_index)
                .cloned()?;
            let matchup_slots = encounter
                .slot_ids
                .iter()
                .map(|slot_id| swiss.slots().at_key(*slot_id).into())
                .collect();
            Some(
                view! {
                    <TournamentGamesDrawer
                        common
                        series=vec![
                            TournamentDrawerSeries {
                                label: None,
                                slots: matchup_slots,
                                participants: encounter.participants,
                                score: None,
                                plan: None,
                            },
                        ]
                        point_system=swiss.configuration().get_untracked().game_point_system
                        close=close_drawer
                    />
                },
            )
        }}
    }
}
