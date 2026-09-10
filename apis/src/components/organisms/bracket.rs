mod connectors;
mod placement_picker;

use crate::{
    common::half_point_text,
    components::{
        molecules::panel::Panel,
        organisms::{
            tournament_encounters::{TournamentDrawerSeries, TournamentGamesDrawer},
            tournament_inspector::TournamentSelection,
        },
    },
    i18n::*,
    providers::{
        EliminationNode,
        EliminationState,
        EliminationStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
    },
    responses::TournamentMemberships,
};
use connectors::Connectors;
use leptos::{
    either::Either,
    html::{Article, Div},
    prelude::*,
};
use leptos_i18n::I18nContext;
use leptos_use::use_element_size;
use placement_picker::PlacementPicker;
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{
        elimination::{Config as EliminationConfig, Resolution, Source, Stage},
        EliminationNodeId,
    },
    tournament_view::{
        EliminationNodeStateResponse,
        EliminationPlayerResultResponse,
        SlotResponse,
    },
    TournamentStatus,
};
use std::{
    array::from_fn,
    collections::{HashMap, HashSet},
    iter::once,
    ops::Range,
    sync::Arc,
};
use tournamint::{elimination as native_elimination, PlayerId};
use uuid::Uuid;

type PossibleEntrants = HashMap<(EliminationNodeId, usize), Vec<Uuid>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BracketPosition {
    node: EliminationNodeId,
    index: usize,
}

#[derive(Clone)]
struct BracketEditingContext {
    order: RwSignal<Vec<Uuid>>,
    seeded_players: Arc<Vec<Uuid>>,
    names: Memo<Arc<HashMap<Uuid, String>>>,
    preview: Memo<Option<(BracketModel, PossibleEntrants)>>,
    possible: Memo<PossibleEntrants>,
    position: RwSignal<Option<BracketPosition>>,
    active_player: Memo<Option<Uuid>>,
}

/// Uses the live bracket graph, including its collapsed bye paths and mobile phases.
#[component]
pub fn BracketPlacementPreview(
    configuration: EliminationConfig,
    seeded_players: Vec<Uuid>,
    order: RwSignal<Vec<Uuid>>,
    position: RwSignal<Option<BracketPosition>>,
    memberships: Signal<TournamentMemberships>,
) -> impl IntoView {
    let i18n = use_i18n();
    let highlighted = RwSignal::new(None);
    let selection = RwSignal::new(None);
    let phase_cursor = RwSignal::new(0);
    let opened_node = RwSignal::new(None);
    let root = NodeRef::<Div>::new();
    provide_context(BracketGamesContext { opened_node });
    let seeded_players = Arc::new(seeded_players);
    let name_seeds = Arc::clone(&seeded_players);
    let names = Memo::new(move |_| {
        Arc::new(
            memberships
                .get()
                .players
                .into_iter()
                .map(|(id, player)| {
                    // TODO: i18n once copy is approved.
                    let name = if player.deleted {
                        format!(
                            "Deleted player #{}",
                            name_seeds.iter().position(|seed| *seed == id).unwrap_or(0) + 1
                        )
                    } else {
                        player.username
                    };
                    (id, name)
                })
                .collect::<HashMap<_, _>>(),
        )
    });
    let preview_seeds = Arc::clone(&seeded_players);
    let preview = Memo::new(move |_| {
        order.with(|order| placement_model(configuration.topology, &preview_seeds, order))
    });
    let possible = Memo::new(move |_| {
        preview.with(|preview| {
            preview
                .as_ref()
                .map(|(_, possible)| possible.clone())
                .unwrap_or_default()
        })
    });
    let active_player = Memo::new(move |_| {
        let position = position.get()?;
        preview.with(|preview| {
            preview
                .as_ref()?
                .0
                .nodes
                .iter()
                .find(|node| node.id == position.node)?
                .entrants[position.index]
        })
    });
    let editor = BracketEditingContext {
        order,
        seeded_players,
        names,
        preview,
        possible,
        position,
        active_player,
    };
    provide_context(editor.clone());
    Effect::watch(
        move || phase_cursor.get(),
        move |_, _, _| position.set(None),
        false,
    );
    view! {
        <div node_ref=root>
            {move || {
                let (model, _) = preview.get()?;
                Some(
                    view! {
                        <EliminationGraph
                            model
                            names=names.get()
                            highlighted
                            selection
                            phase_cursor
                            i18n
                        />
                    },
                )
            }} <Show when=move || position.get().is_some()>
                <PlacementPicker editor=editor.clone() root />
            </Show>
        </div>
    }
}

fn placement_model(
    kind: native_elimination::EliminationKind,
    seeded_players: &[Uuid],
    order: &[Uuid],
) -> Option<(BracketModel, PossibleEntrants)> {
    let seeds = order
        .iter()
        .map(|id| {
            seeded_players
                .iter()
                .position(|seed| seed == id)
                .map(PlayerId::new)
        })
        .collect::<Option<Vec<_>>>()?;
    let definition = native_elimination::topology(kind, &seeds).ok()?;
    let mut facts = native_elimination::initial_facts(&definition);
    let release = native_elimination::next_release(
        &definition,
        &facts,
        native_elimination::EliminationReleaseCadence::DependencyReady,
        &[],
    )
    .ok()?;
    for patch in release.patches {
        let fact = facts.iter_mut().find(|fact| fact.node == patch.node)?;
        fact.state = patch.state;
    }
    let projection = native_elimination::project(&definition, &facts, &[]).ok()?;
    let mut possible = HashMap::new();
    let nodes = projection
        .nodes
        .into_iter()
        .map(|node| {
            let descriptor = node.descriptor;
            for index in 0..2 {
                possible.insert(
                    (descriptor.id, index),
                    node.possible_entrants[index]
                        .iter()
                        .map(|player| seeded_players[player.index()])
                        .collect(),
                );
            }
            let entrants = match node.state {
                native_elimination::EliminationNodeFactState::Active { entrants } => {
                    entrants.map(|player| Some(seeded_players[player.index()]))
                }
                native_elimination::EliminationNodeFactState::Resolved { entrants, .. } => {
                    entrants.map(|player| player.map(|player| seeded_players[player.index()]))
                }
                _ => from_fn(|index| match node.possible_entrants[index].as_slice() {
                    [player] => Some(seeded_players[player.index()]),
                    _ => None,
                }),
            };
            let state = match node.state {
                native_elimination::EliminationNodeFactState::Active { .. } => NodeState::Active,
                native_elimination::EliminationNodeFactState::Resolved {
                    resolution:
                        native_elimination::EliminationNodeResolution::AutomaticAdvance { player },
                    ..
                } => NodeState::Resolved(Resolution::AutomaticAdvance {
                    player: seeded_players[player.index()],
                }),
                native_elimination::EliminationNodeFactState::Resolved { .. } => {
                    NodeState::Resolved(Resolution::Vacancy)
                }
                native_elimination::EliminationNodeFactState::Skipped => {
                    NodeState::Resolved(Resolution::SkippedConditionalBranch)
                }
                native_elimination::EliminationNodeFactState::Planned => NodeState::Planned {
                    conditional: descriptor.stage == Stage::Reset,
                },
            };
            BracketNode {
                id: descriptor.id,
                wave_id: descriptor.scheduled_wave_index,
                stage: descriptor.stage,
                stage_ordinal: descriptor.stage_ordinal,
                sources: descriptor.sources.map(|source| match source {
                    native_elimination::EliminationSource::InitialSeed { seed_index } => {
                        Source::InitialSeed {
                            seed_index: seed_index as u32,
                        }
                    }
                    native_elimination::EliminationSource::WinnerOf(node_id) => {
                        Source::WinnerOf { node_id }
                    }
                    native_elimination::EliminationSource::LoserOf(node_id) => {
                        Source::LoserOf { node_id }
                    }
                    native_elimination::EliminationSource::Vacant => Source::Vacant,
                }),
                entrants,
                projected_entrants: from_fn(|index| {
                    match node.possible_entrants[index].as_slice() {
                        [player] => ProjectedEntrant::Player(seeded_players[player.index()]),
                        [] => ProjectedEntrant::Vacant,
                        _ => ProjectedEntrant::Pending,
                    }
                }),
                seeds: entrants.map(|entrant| {
                    entrant.and_then(|entrant| {
                        seeded_players
                            .iter()
                            .position(|seed| *seed == entrant)
                            .map(|index| index + 1)
                    })
                }),
                aggregate_score: None,
                slot_ids: Vec::new(),
                state,
            }
        })
        .collect();
    Some((
        BracketModel {
            status: TournamentStatus::NotStarted,
            complete: false,
            placements: HashMap::new(),
            nodes,
        },
        possible,
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeState {
    Planned { conditional: bool },
    Active,
    Resolved(Resolution),
}

impl NodeState {
    const fn key(self) -> &'static str {
        match self {
            Self::Planned { conditional: true } => "planned-conditional",
            Self::Planned { conditional: false } => "planned",
            Self::Active => "active",
            Self::Resolved(Resolution::PlayedWinner { .. }) => "resolved-slot",
            Self::Resolved(Resolution::AutomaticAdvance { .. }) => "automatic-advance",
            Self::Resolved(Resolution::Walkover { .. }) => "walkover",
            Self::Resolved(Resolution::Vacancy) => "vacancy",
            Self::Resolved(Resolution::SkippedConditionalBranch) => "skipped-conditional",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BracketNode {
    id: EliminationNodeId,
    wave_id: usize,
    stage: Stage,
    stage_ordinal: usize,
    sources: [Source; 2],
    entrants: [Option<Uuid>; 2],
    projected_entrants: [ProjectedEntrant; 2],
    seeds: [Option<usize>; 2],
    aggregate_score: Option<[u64; 2]>,
    slot_ids: Vec<Uuid>,
    state: NodeState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectedEntrant {
    Player(Uuid),
    Vacant,
    Pending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BracketModel {
    status: TournamentStatus,
    complete: bool,
    placements: HashMap<u32, Vec<Uuid>>,
    nodes: Vec<BracketNode>,
}

#[derive(Clone, Copy)]
struct BracketGamesContext {
    opened_node: RwSignal<Option<EliminationNodeId>>,
}

impl BracketModel {
    fn read(
        status: TournamentStatus,
        memberships: &TournamentMemberships,
        projected_nodes: &[EliminationNode],
        complete: bool,
        player_results: &[EliminationPlayerResultResponse],
    ) -> Self {
        let nodes = projected_nodes
            .iter()
            .map(|node| BracketNode {
                id: node.node_id,
                wave_id: node.wave_index,
                stage: node.stage,
                stage_ordinal: node.stage_ordinal,
                sources: node.sources,
                entrants: node.entrants,
                projected_entrants: from_fn(|index| {
                    if let Some(player) = node.entrants[index] {
                        ProjectedEntrant::Player(player)
                    } else if node.possible_entrants[index].len() == 1 {
                        ProjectedEntrant::Player(node.possible_entrants[index][0])
                    } else if node.possible_entrants[index].is_empty() {
                        ProjectedEntrant::Vacant
                    } else {
                        ProjectedEntrant::Pending
                    }
                }),
                seeds: from_fn(|index| {
                    node.entrants[index]
                        .or_else(|| {
                            (node.possible_entrants[index].len() == 1)
                                .then(|| node.possible_entrants[index][0])
                        })
                        .and_then(|player| memberships.pairing_numbers.get(&player))
                        .map(|seed| seed.saturating_add(1))
                }),
                aggregate_score: node.series.as_ref().map(|series| series.score),
                slot_ids: node
                    .series
                    .iter()
                    .flat_map(|series| &series.sets)
                    .flat_map(|set| &set.slots)
                    .map(|slot| slot.slot_id)
                    .collect(),
                state: match node.state {
                    EliminationNodeStateResponse::Planned { conditional } => {
                        NodeState::Planned { conditional }
                    }
                    EliminationNodeStateResponse::Active => NodeState::Active,
                    EliminationNodeStateResponse::Resolved(resolution) => {
                        NodeState::Resolved(resolution)
                    }
                    EliminationNodeStateResponse::Skipped => {
                        NodeState::Resolved(Resolution::SkippedConditionalBranch)
                    }
                },
            })
            .collect::<Vec<_>>();
        Self {
            status,
            complete,
            placements: player_results.iter().fold(
                HashMap::<u32, Vec<Uuid>>::new(),
                |mut placements, result| {
                    if let Some(place) = result.placement {
                        placements.entry(place).or_default().push(result.player);
                    }
                    placements
                },
            ),
            nodes,
        }
    }

    fn unique_player_at(&self, place: u32) -> Option<Uuid> {
        let players = self.placements.get(&place)?;
        (players.len() == 1).then_some(players[0])
    }
}

fn node_slot_ids(node: &EliminationNode) -> Vec<Uuid> {
    node.series
        .iter()
        .flat_map(|series| &series.sets)
        .flat_map(|set| &set.slots)
        .map(|slot| slot.slot_id)
        .collect()
}

fn model_of(
    status: TournamentStatus,
    memberships: &TournamentMemberships,
    nodes: &[EliminationNode],
    complete: bool,
    player_results: &[EliminationPlayerResultResponse],
) -> Option<BracketModel> {
    (!nodes.is_empty())
        .then(|| BracketModel::read(status, memberships, nodes, complete, player_results))
}

const fn resolution_winner(resolution: Resolution) -> Option<Uuid> {
    match resolution {
        Resolution::PlayedWinner { winner, .. } | Resolution::Walkover { winner, .. } => {
            Some(winner)
        }
        Resolution::AutomaticAdvance { player } => Some(player),
        Resolution::Vacancy | Resolution::SkippedConditionalBranch => None,
    }
}

const fn resolution_loser(resolution: Resolution) -> Option<Uuid> {
    match resolution {
        Resolution::PlayedWinner { loser, .. } => Some(loser),
        Resolution::Walkover { withdrawn, .. } => Some(withdrawn),
        Resolution::AutomaticAdvance { .. }
        | Resolution::Vacancy
        | Resolution::SkippedConditionalBranch => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RouteKind {
    Winner,
    Loser,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GraphEdge {
    from: EliminationNodeId,
    source_row: usize,
    to: EliminationNodeId,
    target_row: usize,
    kind: RouteKind,
    routed_player: Option<Uuid>,
}

fn edges_of(nodes: &[BracketNode]) -> Vec<GraphEdge> {
    let by_id = nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<HashMap<_, _>>();
    nodes
        .iter()
        .flat_map(|node| {
            let by_id = &by_id;
            node.sources
                .iter()
                .enumerate()
                .filter_map(move |(target_row, source)| match source {
                    Source::WinnerOf { node_id } => Some(GraphEdge {
                        from: *node_id,
                        source_row: routed_entrant_row(
                            by_id.get(node_id).copied(),
                            RouteKind::Winner,
                        ),
                        to: node.id,
                        target_row,
                        kind: RouteKind::Winner,
                        routed_player: routed_player(
                            by_id.get(node_id).copied(),
                            RouteKind::Winner,
                        ),
                    }),
                    Source::LoserOf { node_id } => Some(GraphEdge {
                        from: *node_id,
                        source_row: routed_entrant_row(
                            by_id.get(node_id).copied(),
                            RouteKind::Loser,
                        ),
                        to: node.id,
                        target_row,
                        kind: RouteKind::Loser,
                        routed_player: routed_player(by_id.get(node_id).copied(), RouteKind::Loser),
                    }),
                    Source::InitialSeed { .. } | Source::Vacant => None,
                })
        })
        .collect()
}

fn routed_player(node: Option<&BracketNode>, kind: RouteKind) -> Option<Uuid> {
    let resolution = match node?.state {
        NodeState::Resolved(resolution) => resolution,
        NodeState::Planned { .. } | NodeState::Active => return None,
    };
    match kind {
        RouteKind::Winner => resolution_winner(resolution),
        RouteKind::Loser => resolution_loser(resolution),
    }
}

fn routed_entrant_row(node: Option<&BracketNode>, kind: RouteKind) -> usize {
    let routed_player = routed_player(node, kind);

    node.and_then(|node| {
        routed_player.and_then(|player| {
            node.entrants
                .iter()
                .position(|entrant| *entrant == Some(player))
        })
    })
    .unwrap_or(match kind {
        RouteKind::Winner => 0,
        RouteKind::Loser => 1,
    })
}

#[derive(Clone, PartialEq)]
struct ProjectedColumn {
    phase_index: usize,
    wave_id: usize,
    stage: Stage,
    label: String,
    nodes: Vec<BracketNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectedTree {
    Single,
    Upper,
    Lower,
}

fn is_double_elimination(nodes: &[BracketNode]) -> bool {
    nodes.iter().any(|node| {
        matches!(
            node.stage,
            Stage::WinnersRound { .. }
                | Stage::LosersMinor { .. }
                | Stage::LosersMajor { .. }
                | Stage::GrandFinal
                | Stage::Reset
        )
    })
}

fn semantic_phase_count(nodes: &[BracketNode], double_elimination: bool) -> usize {
    if double_elimination {
        nodes
            .iter()
            .filter_map(|node| match node.stage {
                Stage::WinnersRound { round_index } => Some(round_index.saturating_add(1)),
                _ => None,
            })
            .max()
            .unwrap_or(1)
    } else {
        nodes
            .iter()
            .filter_map(|node| match node.stage {
                Stage::SingleRound { round_index } => Some(round_index.saturating_add(2)),
                Stage::SingleFinal | Stage::Bronze => Some(1),
                _ => None,
            })
            .max()
            .unwrap_or(1)
    }
}

fn semantic_phase_label(phase_count: usize, phase_index: usize) -> String {
    let remaining = phase_count.saturating_sub(phase_index);
    match remaining {
        // TODO: i18n once copy is approved.
        1 => String::from("Final"),
        // TODO: i18n once copy is approved.
        2 => String::from("Semifinals"),
        _ => {
            let field_size = u32::try_from(remaining)
                .ok()
                .and_then(|power| 1_usize.checked_shl(power))
                .unwrap_or(usize::MAX);
            // TODO: i18n once copy is approved.
            format!("Round of {field_size}")
        }
    }
}

fn semantic_phase_index(stage: Stage, phase_count: usize) -> Option<usize> {
    match stage {
        Stage::SingleRound { round_index } | Stage::WinnersRound { round_index } => {
            Some(round_index)
        }
        Stage::SingleFinal | Stage::Bronze => Some(phase_count.saturating_sub(1)),
        Stage::LosersMinor { round_index } | Stage::LosersMajor { round_index } => {
            Some(round_index.saturating_add(1))
        }
        Stage::GrandFinal | Stage::Reset => None,
    }
}

fn structural_node(node: &BracketNode) -> bool {
    matches!(
        node.state,
        NodeState::Resolved(
            Resolution::AutomaticAdvance { .. }
                | Resolution::Vacancy
                | Resolution::SkippedConditionalBranch
        )
    ) || node.projected_entrants.contains(&ProjectedEntrant::Vacant)
}

fn projected_stage_column(
    nodes: &[BracketNode],
    phase_index: usize,
    stage: Stage,
    label: String,
) -> Option<ProjectedColumn> {
    let mut stage_nodes = nodes
        .iter()
        .filter(|node| node.stage == stage && !structural_node(node))
        .cloned()
        .collect::<Vec<_>>();
    stage_nodes.sort_by_key(|node| (node.stage_ordinal, node.id));
    let wave_id = stage_nodes.iter().map(|node| node.wave_id).min()?;
    Some(ProjectedColumn {
        phase_index,
        wave_id,
        stage,
        label,
        nodes: stage_nodes,
    })
}

fn projected_columns(
    nodes: &[BracketNode],
    tree: ProjectedTree,
    phase_count: usize,
    phase_start: usize,
    phase_end: usize,
) -> Vec<ProjectedColumn> {
    let mut columns = Vec::new();
    for phase_index in phase_start..phase_end {
        let phase_label = semantic_phase_label(phase_count, phase_index);
        match tree {
            ProjectedTree::Single if phase_index + 1 == phase_count => {
                let mut final_nodes = nodes
                    .iter()
                    .filter(|node| node.stage == Stage::SingleFinal && !structural_node(node))
                    .cloned()
                    .collect::<Vec<_>>();
                final_nodes.sort_by_key(|node| (node.stage_ordinal, node.id));
                if let Some(wave_id) = final_nodes.iter().map(|node| node.wave_id).min() {
                    columns.push(ProjectedColumn {
                        phase_index,
                        wave_id,
                        stage: Stage::SingleFinal,
                        label: phase_label,
                        nodes: final_nodes,
                    });
                }
            }
            ProjectedTree::Single => {
                let stage = Stage::SingleRound {
                    round_index: phase_index,
                };
                if let Some(column) = projected_stage_column(nodes, phase_index, stage, phase_label)
                {
                    columns.push(column);
                }
            }
            ProjectedTree::Upper => {
                let stage = Stage::WinnersRound {
                    round_index: phase_index,
                };
                if let Some(column) = projected_stage_column(nodes, phase_index, stage, phase_label)
                {
                    columns.push(column);
                }
            }
            ProjectedTree::Lower if phase_index > 0 => {
                let round_index = phase_index - 1;
                let labels = if phase_index + 1 == phase_count {
                    // TODO: i18n once copy is approved.
                    [String::from("Lower semifinal"), String::from("Lower final")]
                } else {
                    // TODO: i18n once copy is approved.
                    [format!("{phase_label} · I"), format!("{phase_label} · II")]
                };
                for (stage, label) in [
                    Stage::LosersMinor { round_index },
                    Stage::LosersMajor { round_index },
                ]
                .into_iter()
                .zip(labels)
                {
                    if let Some(column) = projected_stage_column(nodes, phase_index, stage, label) {
                        columns.push(column);
                    }
                }
            }
            ProjectedTree::Lower => {}
        }
    }
    columns
}

fn bracket_dimensions(viewport_width: f64) -> (f64, f64) {
    if viewport_width < 640.0 {
        (((viewport_width - 16.0).max(0.0) / 2.0).min(168.0), 16.0)
    } else {
        (192.0, 40.0)
    }
}

fn projected_columns_width(column_count: usize, viewport_width: f64) -> f64 {
    if column_count == 0 {
        return 0.0;
    }
    let (card_width, gap) = bracket_dimensions(viewport_width);
    card_width * column_count as f64 + gap * column_count.saturating_sub(1) as f64
}

#[derive(Clone)]
struct ProjectedTreeLayout {
    columns: Arc<Vec<ProjectedColumn>>,
    phase_ends: Arc<Vec<usize>>,
}

impl ProjectedTreeLayout {
    fn new(nodes: &[BracketNode], tree: ProjectedTree, phase_count: usize) -> Self {
        let columns = projected_columns(nodes, tree, phase_count, 0, phase_count);
        let phase_ends = (0..=phase_count)
            .map(|phase_end| columns.partition_point(|column| column.phase_index < phase_end))
            .collect();
        Self {
            columns: Arc::new(columns),
            phase_ends: Arc::new(phase_ends),
        }
    }

    fn visible_range(&self, phase_start: usize, viewport_width: f64) -> Range<usize> {
        let phase_count = self.phase_ends.len().saturating_sub(1);
        let phase_start = phase_start.min(phase_count.saturating_sub(1));
        let column_start = self.phase_ends[phase_start];
        let mut column_end = self.phase_ends[phase_start.saturating_add(1).min(phase_count)];
        if viewport_width != 0.0 {
            for candidate_phase_end in (phase_start + 1)..=phase_count {
                let candidate_column_end = self.phase_ends[candidate_phase_end];
                let column_count = candidate_column_end.saturating_sub(column_start);
                if projected_columns_width(column_count, viewport_width) <= viewport_width + 0.5 {
                    column_end = candidate_column_end;
                } else {
                    break;
                }
            }
        }
        column_start..column_end
    }
}

fn player_name(
    i18n: I18nContext<Locale, I18nKeys>,
    names: &HashMap<Uuid, String>,
    player: Uuid,
) -> String {
    names.get(&player).cloned().unwrap_or_else(|| {
        t_string!(i18n, tournaments.view.bracket.unknown_participant).to_string()
    })
}

fn pending_entrant_label(
    i18n: I18nContext<Locale, I18nKeys>,
    source: Source,
    nodes: &[BracketNode],
    visible_nodes: &HashSet<EliminationNodeId>,
) -> String {
    let (node_id, loser_route) = match source {
        Source::WinnerOf { node_id } => (node_id, false),
        Source::LoserOf { node_id } => (node_id, true),
        Source::InitialSeed { .. } | Source::Vacant => {
            return t_string!(i18n, tournaments.view.bracket.to_be_determined).to_string();
        }
    };
    if visible_nodes.contains(&node_id) {
        return t_string!(i18n, tournaments.view.bracket.to_be_determined).to_string();
    }
    let Some(source_node) = nodes.iter().find(|node| node.id == node_id) else {
        return t_string!(i18n, tournaments.view.bracket.to_be_determined).to_string();
    };
    if !loser_route && structural_node(source_node) {
        if let Some(source_index) = source_node
            .projected_entrants
            .iter()
            .position(|entrant| *entrant != ProjectedEntrant::Vacant)
        {
            return pending_entrant_label(
                i18n,
                source_node.sources[source_index],
                nodes,
                visible_nodes,
            );
        }
    }
    let phase_count = semantic_phase_count(nodes, is_double_elimination(nodes));
    let Some(phase_index) = semantic_phase_index(source_node.stage, phase_count) else {
        return t_string!(i18n, tournaments.view.bracket.to_be_determined).to_string();
    };
    let phase = semantic_phase_label(phase_count, phase_index);
    let source_label = match source_node.stage {
        Stage::WinnersRound { .. } => {
            // TODO: i18n once copy is approved.
            format!("Upper {phase}")
        }
        Stage::LosersMinor { .. } if phase_index + 1 == phase_count => {
            // TODO: i18n once copy is approved.
            String::from("Lower semifinal")
        }
        Stage::LosersMajor { .. } if phase_index + 1 == phase_count => {
            // TODO: i18n once copy is approved.
            String::from("Lower final")
        }
        Stage::LosersMinor { .. } => {
            // TODO: i18n once copy is approved.
            format!("Lower {phase} · I")
        }
        Stage::LosersMajor { .. } => {
            // TODO: i18n once copy is approved.
            format!("Lower {phase} · II")
        }
        Stage::SingleRound { .. } | Stage::SingleFinal | Stage::Bronze => phase,
        // TODO: i18n once copy is approved.
        Stage::GrandFinal => String::from("final"),
        Stage::Reset => String::from("second final"),
    };
    let match_number = source_node.stage_ordinal.saturating_add(1);
    if loser_route {
        // TODO: i18n once copy is approved.
        format!("Loser of {source_label} · {match_number}")
    } else {
        // TODO: i18n once copy is approved.
        format!("Winner of {source_label} · {match_number}")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntrantResult {
    Winner,
    Loser,
    Advanced,
    Withdrawn,
}

impl EntrantResult {
    const fn emphasized(self) -> bool {
        matches!(self, Self::Winner | Self::Advanced)
    }

    const fn faded(self) -> bool {
        matches!(self, Self::Loser | Self::Withdrawn)
    }
}

fn entrant_result(state: NodeState, player: Option<Uuid>) -> Option<EntrantResult> {
    let player = player?;
    match state {
        NodeState::Resolved(Resolution::PlayedWinner { winner, loser }) => {
            if player == winner {
                Some(EntrantResult::Winner)
            } else if player == loser {
                Some(EntrantResult::Loser)
            } else {
                None
            }
        }
        NodeState::Resolved(Resolution::AutomaticAdvance { player: advanced }) => {
            (player == advanced).then_some(EntrantResult::Advanced)
        }
        NodeState::Resolved(Resolution::Walkover { winner, withdrawn }) => {
            if player == winner {
                Some(EntrantResult::Advanced)
            } else if player == withdrawn {
                Some(EntrantResult::Withdrawn)
            } else {
                None
            }
        }
        NodeState::Planned { .. }
        | NodeState::Active
        | NodeState::Resolved(Resolution::Vacancy)
        | NodeState::Resolved(Resolution::SkippedConditionalBranch) => None,
    }
}

fn championship_nodes(nodes: &[BracketNode]) -> Vec<BracketNode> {
    [Stage::GrandFinal, Stage::Reset]
        .into_iter()
        .filter_map(|stage| nodes.iter().find(|node| node.stage == stage).cloned())
        .collect()
}

#[component]
pub fn Bracket(
    common: Store<TournamentCommon>,
    elimination: Store<EliminationState>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let highlighted = RwSignal::new(None::<Uuid>);
    let phase_cursor = RwSignal::new(0_usize);
    let opened_node = RwSignal::new(None::<EliminationNodeId>);
    let close_drawer = Callback::new(move |()| opened_node.set(None));
    provide_context(BracketGamesContext { opened_node });
    view! {
        <Panel title=move || common.lifecycle().get().name class="min-w-0">
            {move || {
                let memberships = common.memberships().get();
                let status = common.lifecycle().get().status;
                let names = Arc::new(
                    memberships
                        .players
                        .iter()
                        .map(|(id, player)| (*id, player.username.clone()))
                        .collect::<HashMap<_, _>>(),
                );
                let model = model_of(
                    status,
                    &memberships,
                    &elimination.nodes().get(),
                    elimination.complete().get(),
                    &elimination.player_results().get(),
                );
                match model {
                    Some(model) => {
                        Either::Left(
                            view! {
                                <EliminationGraph
                                    model
                                    names
                                    highlighted
                                    selection
                                    phase_cursor
                                    i18n
                                />
                            },
                        )
                    }
                    None => {
                        Either::Right(
                            view! {
                                <p class="py-6 text-sm text-center text-gray-500 dark:text-gray-400">
                                    {t!(i18n, tournaments.view.bracket.unavailable)}
                                </p>
                            },
                        )
                    }
                }
            }}
        </Panel>
        {move || {
            let node_id = opened_node.get()?;
            let nodes = elimination.nodes().get();
            let node = nodes.iter().find(|node| node.node_id == node_id)?;
            let championship = matches!(node.stage, Stage::GrandFinal | Stage::Reset);
            let configuration = elimination.configuration().get();
            let series = once(node)
                .filter_map(|node| {
                    let slots: Vec<ArcField<SlotResponse>> = node_slot_ids(node)
                        .into_iter()
                        .map(|slot_id| elimination.slots().at_key(slot_id).into())
                        .collect();
                    let first = slots.first()?.try_get_untracked()?;
                    let left = node.entrants[0].unwrap_or_else(|| first.white());
                    let right = node
                        .entrants[1]
                        .unwrap_or_else(|| {
                            if first.white() == left { first.black() } else { first.white() }
                        });
                    let label = championship
                        .then_some(
                            if node.stage == Stage::Reset { "Second final" } else { "Final" },
                        );
                    Some(TournamentDrawerSeries {
                        label,
                        slots,
                        participants: [left, right],
                        score: node.series.as_ref().map(|series| series.score),
                        plan: Some(configuration.effective_plan(node.stage).clone()),
                    })
                })
                .collect::<Vec<_>>();
            if series.is_empty() {
                return None;
            }
            Some(
                // TODO: i18n once copy is approved.
                view! {
                    <TournamentGamesDrawer
                        common
                        series
                        point_system=tournamint::PointSystem::STANDARD
                        close=close_drawer
                    />
                },
            )
        }}
    }
}

#[component]
fn EliminationGraph(
    model: BracketModel,
    names: Arc<HashMap<Uuid, String>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    phase_cursor: RwSignal<usize>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let editing = use_context::<BracketEditingContext>().is_some();
    let viewport = NodeRef::<Div>::new();
    let viewport_size = use_element_size(viewport);
    let double_elimination = is_double_elimination(&model.nodes);
    let phase_count = semantic_phase_count(&model.nodes, double_elimination);
    Effect::new(move |_| {
        if phase_cursor.get() >= phase_count {
            phase_cursor.set(phase_count.saturating_sub(1));
        }
    });
    let all_nodes = Arc::new(model.nodes.clone());
    let upper_layout = double_elimination
        .then(|| ProjectedTreeLayout::new(&all_nodes, ProjectedTree::Upper, phase_count));
    let lower_layout = double_elimination
        .then(|| ProjectedTreeLayout::new(&all_nodes, ProjectedTree::Lower, phase_count));
    let single_layout = (!double_elimination)
        .then(|| ProjectedTreeLayout::new(&all_nodes, ProjectedTree::Single, phase_count));
    let dimensions = Memo::new(move |_| bracket_dimensions(viewport_size.width.get()));
    let mut lower_exists = false;
    let mut grand_final = None;
    let mut bronze = None;
    for node in all_nodes.iter() {
        match node.stage {
            Stage::LosersMinor { .. } | Stage::LosersMajor { .. } => lower_exists = true,
            Stage::GrandFinal if grand_final.is_none() => grand_final = Some(node.clone()),
            Stage::Bronze if bronze.is_none() && !structural_node(node) => {
                bronze = Some(node.clone());
            }
            _ => {}
        }
    }
    let champion = model.unique_player_at(1);
    let runner_up = model.unique_player_at(2);
    let third_place = model.unique_player_at(3);
    let status = model.status;
    let complete = model.complete;
    let projection_label = move || {
        if status == TournamentStatus::NotStarted {
            // TODO: i18n once copy is approved.
            String::from("Bracket placement")
        } else if status == TournamentStatus::Finished {
            // TODO: i18n once copy is approved.
            String::from("Final outcome")
        } else if complete {
            // TODO: i18n once copy is approved.
            String::from("Bracket complete")
        } else {
            t_string!(i18n, tournaments.view.bracket.completion.nodes_in_progress).to_string()
        }
    };
    view! {
        <div class="flex flex-wrap gap-2 justify-between items-center mb-3" class:hidden=editing>
            <p class="text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                {projection_label}
            </p>
            {(status == TournamentStatus::Finished)
                .then_some(champion)
                .flatten()
                .map(|champion| {
                    view! {
                        <FinalOutcome
                            champion
                            runner_up
                            third_place
                            names=Arc::clone(&names)
                            selection
                        />
                    }
                })}
        </div>
        <div
            node_ref=viewport
            class="overflow-hidden max-w-full"
            style=move || {
                let (card_width, gap) = dimensions.get();
                format!("--bracket-card-width:{card_width}px;--bracket-gap:{gap}px")
            }
            data-testid="bracket-viewport"
        >
            <PhaseRail phase_count phase_cursor />
            {if double_elimination {
                let upper_title = String::from("Upper bracket");
                let lower_title = String::from("Lower bracket");
                let lower_empty_label = String::from("The lower bracket begins in the next phase.");
                // TODO: i18n once copy is approved.
                // TODO: i18n once copy is approved.
                // TODO: i18n once copy is approved.
                view! {
                    <div class="space-y-8">
                        <TreeSection
                            title=upper_title
                            section_key="upper"
                            layout=upper_layout.unwrap()
                            viewport_width=viewport_size.width
                            phase_cursor
                            names=Arc::clone(&names)
                            all_nodes=Arc::clone(&all_nodes)
                            highlighted
                            selection
                            i18n
                        />
                        {lower_exists
                            .then(|| {
                                view! {
                                    <TreeSection
                                        title=lower_title
                                        section_key="lower"
                                        layout=lower_layout.unwrap()
                                        viewport_width=viewport_size.width
                                        phase_cursor
                                        empty_label=lower_empty_label
                                        names=Arc::clone(&names)
                                        all_nodes=Arc::clone(&all_nodes)
                                        highlighted
                                        selection
                                        i18n
                                    />
                                }
                            })}
                        {grand_final
                            .as_ref()
                            .map(|_| {
                                view! {
                                    <ChampionshipSection
                                        names=Arc::clone(&names)
                                        all_nodes=Arc::clone(&all_nodes)
                                        highlighted
                                        selection
                                        i18n
                                    />
                                }
                            })}
                    </div>
                }
                    .into_any()
            } else {
                let title = String::from("Bracket");
                // TODO: i18n once copy is approved.
                view! {
                    <div class="space-y-8">
                        <TreeSection
                            title
                            section_key="single"
                            layout=single_layout.unwrap()
                            viewport_width=viewport_size.width
                            phase_cursor
                            names=Arc::clone(&names)
                            all_nodes=Arc::clone(&all_nodes)
                            highlighted
                            selection
                            i18n
                        />
                        {bronze
                            .clone()
                            .map(|node| {
                                view! {
                                    <BronzeSection
                                        node
                                        names=Arc::clone(&names)
                                        all_nodes=Arc::clone(&all_nodes)
                                        highlighted
                                        selection
                                        i18n
                                    />
                                }
                            })}
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
fn FinalOutcome(
    champion: Uuid,
    runner_up: Option<Uuid>,
    third_place: Option<Uuid>,
    names: Arc<HashMap<Uuid, String>>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let champion_name = player_name(i18n, &names, champion);
    let runner_up_name = runner_up.map(|player| player_name(i18n, &names, player));
    let third_place_name = third_place.map(|player| player_name(i18n, &names, player));
    view! {
        <div class="flex flex-wrap gap-y-1 gap-x-4 text-sm text-gray-900 dark:text-gray-100">
            // TODO: i18n once copy is approved.
            <p>
                <strong>"Champion: "</strong>
                <button
                    type="button"
                    class="font-semibold hover:text-pillbug-teal"
                    on:click=move |_| selection.set(Some(TournamentSelection::Player(champion)))
                >
                    {champion_name}
                </button>
            </p>
            {runner_up
                .zip(runner_up_name)
                .map(|(player, name)| {
                    view! {
                        // TODO: i18n once copy is approved.
                        <p>
                            <strong>"Runner-up: "</strong>
                            <button
                                type="button"
                                class="font-semibold hover:text-pillbug-teal"
                                on:click=move |_| {
                                    selection.set(Some(TournamentSelection::Player(player)))
                                }
                            >
                                {name}
                            </button>
                        </p>
                    }
                })}
            {third_place
                .zip(third_place_name)
                .map(|(player, name)| {
                    view! {
                        // TODO: i18n once copy is approved.
                        <p>
                            <strong>"Third place: "</strong>
                            <button
                                type="button"
                                class="font-semibold hover:text-pillbug-teal"
                                on:click=move |_| {
                                    selection.set(Some(TournamentSelection::Player(player)))
                                }
                            >
                                {name}
                            </button>
                        </p>
                    }
                })}
        </div>
    }
}

#[component]
fn PhaseRail(phase_count: usize, phase_cursor: RwSignal<usize>) -> impl IntoView {
    let phases = (0..phase_count)
        .map(|phase_index| (phase_index, semantic_phase_label(phase_count, phase_index)))
        .collect::<Vec<_>>();
    // TODO: i18n once copy is approved.
    let rail_label = String::from("Bracket phase");
    // TODO: i18n once copy is approved.
    let earlier_label = String::from("‹ Earlier");
    // TODO: i18n once copy is approved.
    let later_label = String::from("Later ›");
    view! {
        <nav
            class="flex gap-2 justify-between items-center mb-6"
            data-testid="bracket-phase-rail"
            aria-label=rail_label
        >
            <button
                type="button"
                class="ui-button ui-button-ghost ui-button-sm shrink-0"
                prop:disabled=move || phase_cursor.get() == 0
                on:click=move |_| phase_cursor.update(|cursor| *cursor = cursor.saturating_sub(1))
            >
                {earlier_label}
            </button>
            <div class="flex flex-wrap gap-1 justify-center min-w-0">
                {phases
                    .into_iter()
                    .map(|(phase_index, label)| {
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    if phase_cursor.get() == phase_index {
                                        "ui-button ui-button-primary ui-button-sm"
                                    } else {
                                        "hidden ui-button ui-button-ghost ui-button-sm sm:inline-flex"
                                    }
                                }
                                aria-current=move || {
                                    (phase_cursor.get() == phase_index).then_some("step")
                                }
                                data-bracket-phase=phase_index.to_string()
                                on:click=move |_| phase_cursor.set(phase_index)
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <button
                type="button"
                class="ui-button ui-button-ghost ui-button-sm shrink-0"
                prop:disabled=move || { phase_cursor.get().saturating_add(1) >= phase_count }
                on:click=move |_| {
                    phase_cursor
                        .update(|cursor| *cursor = cursor.saturating_add(1).min(phase_count - 1));
                }
            >
                {later_label}
            </button>
        </nav>
    }
}

#[component]
fn TreeSection(
    title: String,
    section_key: &'static str,
    layout: ProjectedTreeLayout,
    viewport_width: Signal<f64>,
    phase_cursor: RwSignal<usize>,
    #[prop(optional)] empty_label: Option<String>,
    names: Arc<HashMap<Uuid, String>>,
    all_nodes: Arc<Vec<BracketNode>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let container = NodeRef::<Div>::new();
    let columns = Arc::clone(&layout.columns);
    let visible_range =
        Memo::new(move |_| layout.visible_range(phase_cursor.get(), viewport_width.get()));
    let visible_columns = Arc::clone(&columns);
    let visible_nodes = Memo::new(move |_| {
        Arc::new(
            visible_columns[visible_range.get()]
                .iter()
                .flat_map(|column| column.nodes.iter().map(|node| node.id))
                .collect::<HashSet<_>>(),
        )
    });
    let card_refs = columns
        .iter()
        .flat_map(|column| column.nodes.iter())
        .map(|node| (node.id, NodeRef::<Article>::new()))
        .collect::<HashMap<_, _>>();
    let row_refs = card_refs
        .keys()
        .map(|id| (*id, from_fn(|_| NodeRef::<Div>::new())))
        .collect::<HashMap<_, [NodeRef<Div>; 2]>>();
    let edges = edges_of(&all_nodes)
        .into_iter()
        .filter(|edge| edge.kind == RouteKind::Winner)
        .filter(|edge| card_refs.contains_key(&edge.from) && card_refs.contains_key(&edge.to))
        .collect::<Vec<_>>();
    view! {
        <section class="min-w-0" data-bracket-section=section_key>
            <h2 class="mb-3 text-xs font-bold tracking-wider text-gray-500 uppercase dark:text-gray-400">
                {title}
            </h2>
            <Show when=move || {
                visible_range.with(Range::is_empty)
            }>
                {empty_label
                    .clone()
                    .map(|label| {
                        view! {
                            <p class="py-5 text-sm text-gray-500 dark:text-gray-400">{label}</p>
                        }
                    })}
            </Show>
            <div node_ref=container class="relative py-2">
                <Connectors
                    edges
                    container
                    rows=row_refs.clone()
                    visible_nodes
                    highlighted
                    selection
                />
                <div class="flex relative z-10 items-stretch gap-(--bracket-gap)">
                    <For
                        each=move || visible_range.get()
                        key=|index| *index
                        children=move |index| {
                            let column = columns[index].clone();
                            view! {
                                <section
                                    class="flex flex-col w-(--bracket-card-width) shrink-0"
                                    data-elimination-phase=column.phase_index.to_string()
                                    data-elimination-wave=column.wave_id.to_string()
                                    data-elimination-stage=format!("{:?}", column.stage)
                                >
                                    <h3 class="mb-3 text-sm font-bold text-center text-gray-700 dark:text-gray-200">
                                        {column.label}
                                    </h3>
                                    <div class="flex flex-col flex-1 gap-2.5 justify-around ui-bracket-feed">
                                        {column
                                            .nodes
                                            .into_iter()
                                            .map(|node| {
                                                let card_ref = card_refs[&node.id];
                                                let row_refs = row_refs[&node.id];
                                                view! {
                                                    <NodeCard
                                                        node
                                                        card_ref
                                                        row_refs
                                                        names=Arc::clone(&names)
                                                        all_nodes=Arc::clone(&all_nodes)
                                                        visible_nodes=visible_nodes.into()
                                                        highlighted
                                                        selection
                                                        i18n
                                                    />
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </section>
                            }
                        }
                    />
                </div>
            </div>
        </section>
    }
}

#[component]
fn ChampionshipSection(
    names: Arc<HashMap<Uuid, String>>,
    all_nodes: Arc<Vec<BracketNode>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let finals = championship_nodes(&all_nodes);
    let visible_nodes = Signal::stored(Arc::new(
        finals.iter().map(|node| node.id).collect::<HashSet<_>>(),
    ));
    view! {
        <section
            class="pt-5 border-t border-gray-200 dark:border-gray-700"
            data-bracket-section="championship"
        >
            <div class="flex flex-col items-start sm:flex-row sm:items-center">
                {finals
                    .into_iter()
                    .enumerate()
                    .map(|(index, node)| {
                        let second_final = node.stage == Stage::Reset;
                        let skipped = matches!(
                            node.state,
                            NodeState::Resolved(Resolution::SkippedConditionalBranch)
                        );
                        let pending = matches!(node.state, NodeState::Planned { .. });
                        let card_ref = NodeRef::<Article>::new();
                        view! {
                            <Show when=move || { index > 0 }>
                                <div class="ml-8 h-7 border-l-2 border-gray-300 sm:ml-0 sm:w-8 sm:h-0 sm:border-l-0 sm:border-t-2 dark:border-gray-600" />
                            </Show>
                            <div
                                class="w-(--bracket-card-width)"
                                data-final-stage=if second_final { "second-final" } else { "final" }
                            >
                                // TODO: i18n once copy is approved.
                                <h2 class="mb-2 text-sm font-semibold">
                                    {if second_final { "Second final" } else { "Final" }}
                                    <span class="ml-2 text-xs font-normal text-gray-500 dark:text-gray-400">
                                        {if skipped {
                                            "Not needed"
                                        } else if second_final && pending {
                                            "If needed"
                                        } else {
                                            ""
                                        }}
                                    </span>
                                </h2>
                                <NodeCard
                                    node
                                    card_ref
                                    names=Arc::clone(&names)
                                    all_nodes=Arc::clone(&all_nodes)
                                    visible_nodes
                                    highlighted
                                    selection
                                    i18n
                                />
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </section>
    }
}

#[component]
fn BronzeSection(
    node: BracketNode,
    names: Arc<HashMap<Uuid, String>>,
    all_nodes: Arc<Vec<BracketNode>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let card_ref = NodeRef::<Article>::new();
    let visible_nodes = Signal::stored(Arc::new(HashSet::from([node.id])));
    // TODO: i18n once copy is approved.
    let title = String::from("Third place");
    view! {
        <section
            class="pt-5 border-t border-gray-200 dark:border-gray-700"
            data-bracket-section="bronze"
        >
            <h2 class="mb-3 text-xs font-bold tracking-wider text-gray-500 uppercase dark:text-gray-400">
                {title}
            </h2>
            <div class="w-(--bracket-card-width)">
                <NodeCard node card_ref names all_nodes visible_nodes highlighted selection i18n />
            </div>
        </section>
    }
}

#[component]
fn NodeCard(
    node: BracketNode,
    card_ref: NodeRef<Article>,
    #[prop(default = from_fn(|_| NodeRef::new()))] row_refs: [NodeRef<Div>; 2],
    names: Arc<HashMap<Uuid, String>>,
    all_nodes: Arc<Vec<BracketNode>>,
    visible_nodes: Signal<Arc<HashSet<EliminationNodeId>>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let games = expect_context::<BracketGamesContext>();
    let editing = use_context::<BracketEditingContext>();
    let node_id = node.id.value();
    let state = node.state;
    let state_key = state.key();
    let active = matches!(state, NodeState::Active);
    let base_card_class = if active {
        "overflow-hidden w-full rounded-lg border-2 border-pillbug-teal bg-white shadow-sm dark:bg-surface-row-even"
    } else if matches!(
        state,
        NodeState::Resolved(Resolution::SkippedConditionalBranch | Resolution::Vacancy)
    ) {
        "overflow-hidden w-full rounded-lg border border-dashed border-gray-300 bg-white/70 shadow-sm dark:border-gray-700 dark:bg-surface-row-even/70"
    } else {
        "overflow-hidden w-full rounded-lg border border-gray-300 bg-white shadow-sm dark:border-gray-700 dark:bg-surface-row-even"
    };
    let node_identity = node.id;
    let openable = !node.slot_ids.is_empty();
    let node_entrants = node.entrants;
    let card_class = move || {
        let selection_state = selection.get();
        let active_player = editing
            .as_ref()
            .and_then(|editor| editor.active_player.get())
            .or_else(|| highlighted.get())
            .or_else(|| selection_state.map(TournamentSelection::player));
        let emphasis = match active_player {
            Some(player)
                if node_entrants.contains(&Some(player))
                    || editing.as_ref().is_some_and(|editor| {
                        (0..2).any(|index| {
                            editor.possible.with(|possible| {
                                possible
                                    .get(&(node_identity, index))
                                    .is_some_and(|candidates| candidates.contains(&player))
                            })
                        })
                    }) =>
            {
                " ring-2 ring-pillbug-teal"
            }
            Some(_) => " opacity-45",
            None => "",
        };
        format!("{base_card_class}{emphasis} transition")
    };

    view! {
        <article
            node_ref=card_ref
            class=card_class
            data-elimination-node=node_id.to_string()
            data-node-state=state_key
        >
            {if openable {
                Either::Left(
                    view! {
                        <button
                            type="button"
                            class="block w-full cursor-pointer"
                            on:click=move |_| games.opened_node.set(Some(node_identity))
                        >
                            <NodeContent
                                node
                                row_refs
                                names
                                all_nodes
                                visible_nodes
                                highlighted
                                selection
                                i18n
                            />
                        </button>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <NodeContent
                            node
                            row_refs
                            names
                            all_nodes
                            visible_nodes
                            highlighted
                            selection
                            i18n
                        />
                    },
                )
            }}
        </article>
    }
}

#[component]
fn NodeContent(
    node: BracketNode,
    row_refs: [NodeRef<Div>; 2],
    names: Arc<HashMap<Uuid, String>>,
    all_nodes: Arc<Vec<BracketNode>>,
    visible_nodes: Signal<Arc<HashSet<EliminationNodeId>>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    let editing = use_context::<BracketEditingContext>();
    let node_id = node.id;
    let state = node.state;
    let sources = node.sources;
    let entrants = node.entrants;
    let projected_entrants = node.projected_entrants;
    let seeds = node.seeds;
    let aggregate_score = node.aggregate_score;
    view! {
        <div class="divide-y divide-gray-200 dark:divide-gray-700">
            {[0_usize, 1_usize]
                .into_iter()
                .map(|index| {
                    let player = entrants[index];
                    let editing = editing.clone();
                    let preview = editing.is_some();
                    let editable = preview && player.is_some()
                        && matches!(state, NodeState::Active | NodeState::Planned { .. });
                    let tooltip = editing
                        .as_ref()
                        .map(|editor| {
                            editor
                                .possible
                                .with_untracked(|possible| {
                                    possible
                                        .get(&(node_id, index))
                                        .into_iter()
                                        .flatten()
                                        .filter_map(|id| {
                                            editor.names.get_untracked().get(id).cloned()
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                        });
                    let result = entrant_result(state, player);
                    let seed = seeds[index];
                    let entrant_names = Arc::clone(&names);
                    let entrant_nodes = Arc::clone(&all_nodes);
                    let entrant_visible_nodes = if preview {
                        Signal::stored(Arc::new(HashSet::new()))
                    } else {
                        visible_nodes
                    };
                    let name = move || match projected_entrants[index] {
                        ProjectedEntrant::Player(player) => {
                            player_name(i18n, &entrant_names, player)
                        }
                        ProjectedEntrant::Vacant => {
                            t_string!(i18n, tournaments.view.bracket.vacant).to_string()
                        }
                        ProjectedEntrant::Pending => {
                            pending_entrant_label(
                                i18n,
                                sources[index],
                                &entrant_nodes,
                                &entrant_visible_nodes.get(),
                            )
                        }
                    };
                    let score = aggregate_score
                        .map(|score| half_point_text(score[index]))
                        .unwrap_or_else(|| String::from("—"));
                    let target = BracketPosition {
                        node: node_id,
                        index,
                    };
                    let selected = editing.as_ref().map(|editor| editor.position);
                    let row_class = move || {
                        let highlight = player.is_some()
                            && (highlighted.get() == player
                                || player
                                    .is_some_and(|player| {
                                        selection.get().map(TournamentSelection::player)
                                            == Some(player)
                                    }));
                        let emphasis = result.is_some_and(EntrantResult::emphasized);
                        let faded = result.is_some_and(EntrantResult::faded);
                        let background = if highlight
                            || selected.is_some_and(|position| position.get() == Some(target))
                        {
                            "bg-pillbug-teal/20"
                        } else if index == 0 {
                            "bg-odd-light dark:bg-surface-row-odd"
                        } else {
                            "bg-even-light dark:bg-surface-row-even"
                        };
                        let weight = if emphasis {
                            "font-bold text-gray-900 dark:text-gray-100"
                        } else if faded {
                            "text-gray-400 dark:text-gray-500"
                        } else {
                            "text-gray-700 dark:text-gray-300"
                        };
                        let height = if preview { "h-10 sm:h-8" } else { "h-7" };
                        format!(
                            "flex relative gap-2 justify-between items-center px-2 w-full text-left {height} {background} {weight}",
                        )
                    };
                    view! {
                        <div
                            node_ref=row_refs[index]
                            class=row_class
                            data-entrant-index=index.to_string()
                            title=tooltip
                            on:mouseenter=move |_| highlighted.set(player)
                            on:mouseleave=move |_| highlighted.set(None)
                        >
                            {match player {
                                Some(_) => {
                                    Either::Left(
                                        view! {
                                            <span
                                                class="flex gap-1 items-center min-w-0"
                                                class:hidden=editable
                                            >
                                                {seed
                                                    .map(|seed| {
                                                        view! {
                                                            <span class="text-xs tabular-nums text-gray-500 shrink-0">
                                                                {format!("#{seed}")}
                                                            </span>
                                                        }
                                                    })}
                                                <span
                                                    class="min-w-0 text-sm truncate"
                                                    class:hidden=editable
                                                >
                                                    {name}
                                                </span>
                                                {result
                                                    .filter(|result| *result == EntrantResult::Withdrawn)
                                                    .map(|_| {
                                                        view! {
                                                            // TODO: i18n once copy is approved.
                                                            <span class="font-semibold text-amber-700 uppercase dark:text-amber-300 text-[10px] shrink-0">
                                                                "Withdrawn"
                                                            </span>
                                                        }
                                                    })}
                                            </span>
                                        },
                                    )
                                }
                                None => {
                                    Either::Right(
                                        view! {
                                            <span class="flex gap-1 items-center min-w-0">
                                                {seed
                                                    .map(|seed| {
                                                        view! {
                                                            <span class="text-xs tabular-nums text-gray-500 shrink-0">
                                                                {format!("#{seed}")}
                                                            </span>
                                                        }
                                                    })}
                                                <span
                                                    class="min-w-0 text-sm truncate"
                                                    class:hidden=editable
                                                >
                                                    {name}
                                                </span>
                                            </span>
                                        },
                                    )
                                }
                            }}
                            {editing
                                .filter(|_| editable)
                                .map(|editor| {
                                    let position = editor.position;
                                    let label = names
                                        .get(&player.unwrap())
                                        .cloned()
                                        .unwrap_or_default();
                                    // TODO: i18n once copy is approved.
                                    view! {
                                        <button
                                            type="button"
                                            class="flex absolute inset-0 gap-2 items-center px-2 w-full text-left cursor-pointer hover:bg-pillbug-teal/10 focus-visible:outline-2 focus-visible:outline-pillbug-teal focus-visible:-outline-offset-2"
                                            aria-label=format!("Assign player: {label}")
                                            aria-haspopup="dialog"
                                            aria-expanded=move || {
                                                (position.get() == Some(target)).to_string()
                                            }
                                            aria-controls="bracket-player-picker"
                                            data-bracket-position=""
                                            class:ring-2=move || position.get() == Some(target)
                                            class:ring-inset=move || position.get() == Some(target)
                                            class:ring-pillbug-teal=move || {
                                                position.get() == Some(target)
                                            }
                                            on:focus=move |_| highlighted.set(player)
                                            on:blur=move |_| highlighted.set(None)
                                            on:click=move |_| position.set(Some(target))
                                        >
                                            <span class="text-xs tabular-nums text-gray-500 shrink-0">
                                                {seed.map(|seed| format!("#{seed}"))}
                                            </span>
                                            <span class="min-w-0 text-sm truncate" title=label.clone()>
                                                {label.clone()}
                                            </span>
                                        </button>
                                    }
                                })}
                            <span class="font-semibold tabular-nums shrink-0" class:hidden=preview>
                                {score}
                            </span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_preview_exposes_all_33_players_without_moving_bye_sources() {
        let players = (1..=33).map(Uuid::from_u128).collect::<Vec<_>>();
        let mut order = players.clone();
        order.swap(0, 32);
        let kind = native_elimination::EliminationKind::Single { bronze: false };
        let (default, _) = placement_model(kind, &players, &players).unwrap();
        let (custom, _) = placement_model(kind, &players, &order).unwrap();
        assert_eq!(
            default
                .nodes
                .iter()
                .map(|node| node.sources)
                .collect::<Vec<_>>(),
            custom
                .nodes
                .iter()
                .map(|node| node.sources)
                .collect::<Vec<_>>()
        );
        let editable = custom
            .nodes
            .iter()
            .filter(|node| matches!(node.state, NodeState::Active | NodeState::Planned { .. }))
            .flat_map(|node| node.entrants.into_iter().flatten())
            .collect::<Vec<_>>();
        assert_eq!(editable.len(), 33);
        assert_eq!(
            editable.into_iter().collect::<HashSet<_>>(),
            players.into_iter().collect()
        );
    }

    fn championship_node(
        id: usize,
        stage: Stage,
        state: EliminationNodeStateResponse,
        entrants: [Uuid; 2],
    ) -> EliminationNode {
        EliminationNode {
            node_id: EliminationNodeId::new(id),
            wave_index: id,
            stage,
            stage_ordinal: 0,
            sources: [Source::Vacant; 2],
            entrants: entrants.map(Some),
            possible_entrants: entrants.map(|player| vec![player]),
            state,
            series: None,
        }
    }

    #[test]
    fn championship_keeps_first_result_when_second_final_changes_state() {
        let entrants = [Uuid::from_u128(1), Uuid::from_u128(2)];
        let first = championship_node(
            1,
            Stage::GrandFinal,
            EliminationNodeStateResponse::Resolved(Resolution::PlayedWinner {
                winner: entrants[1],
                loser: entrants[0],
            }),
            entrants,
        );
        for state in [
            EliminationNodeStateResponse::Planned { conditional: true },
            EliminationNodeStateResponse::Active,
            EliminationNodeStateResponse::Resolved(Resolution::PlayedWinner {
                winner: entrants[0],
                loser: entrants[1],
            }),
            EliminationNodeStateResponse::Skipped,
        ] {
            let second = championship_node(2, Stage::Reset, state, [entrants[1], entrants[0]]);
            let model = BracketModel::read(
                TournamentStatus::InProgress,
                &TournamentMemberships::default(),
                &[second.clone(), first.clone()],
                false,
                &[],
            );
            let finals = championship_nodes(&model.nodes);
            assert_eq!(finals.len(), 2);
            assert_eq!(finals[0].id, first.node_id);
            assert_eq!(finals[0].entrants, first.entrants);
            assert_eq!(
                finals[0].state,
                NodeState::Resolved(Resolution::PlayedWinner {
                    winner: entrants[1],
                    loser: entrants[0],
                })
            );
            assert_eq!(finals[1].id, second.node_id);
            assert_eq!(finals[1].entrants, second.entrants);
        }
    }
}
