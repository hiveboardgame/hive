use super::{BracketEditingContext, GraphEdge, RouteKind};
use crate::components::organisms::tournament_inspector::TournamentSelection;
use leptos::{
    html::Div,
    leptos_dom::helpers::{request_animation_frame_with_handle, AnimationFrameRequestHandle},
    prelude::*,
};
use leptos_use::use_resize_observer;
use shared_types::tournament::EliminationNodeId;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;

#[component]
pub(super) fn Connectors(
    edges: Vec<GraphEdge>,
    container: NodeRef<Div>,
    rows: HashMap<EliminationNodeId, [NodeRef<Div>; 2]>,
    visible_nodes: Memo<Arc<HashSet<EliminationNodeId>>>,
    highlighted: RwSignal<Option<Uuid>>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let editing = use_context::<BracketEditingContext>();
    let paths = ArcRwSignal::new(vec![String::new(); edges.len()]);
    let mounted = ArcRwSignal::new(true);
    let pending = ArcStoredValue::new(None::<AnimationFrameRequestHandle>);
    let cleanup_mounted = mounted.clone();
    let cleanup_pending = pending.clone();
    on_cleanup(move || {
        cleanup_mounted.set(false);
        if let Some(handle) = cleanup_pending.get_value() {
            handle.cancel();
        }
    });

    let targets = rows
        .values()
        .flatten()
        .copied()
        .chain([container])
        .collect::<Vec<_>>();
    let measured_edges = Arc::new(edges.clone());
    let measured_paths = paths.clone();
    use_resize_observer(targets, move |_, _| {
        if !mounted.get_untracked() {
            return;
        }
        if let Some(handle) = pending.get_value() {
            handle.cancel();
        }
        let Some(root) = container.get_untracked() else {
            return;
        };
        // Only DOM references and Arc-backed state cross the frame boundary. A rebuilt
        // section can dispose its NodeRefs before this callback gets a chance to run.
        let elements = rows
            .iter()
            .filter(|(id, _)| visible_nodes.with_untracked(|visible| visible.contains(id)))
            .flat_map(|(id, rows)| {
                rows.iter().enumerate().filter_map(|(index, row)| {
                    row.get_untracked()
                        .filter(|element| element.is_connected())
                        .map(|element| ((*id, index), element))
                })
            })
            .collect::<Vec<_>>();
        let mounted = mounted.clone();
        let frame_pending = pending.clone();
        let paths = measured_paths.clone();
        let edges = Arc::clone(&measured_edges);
        let handle = request_animation_frame_with_handle(move || {
            frame_pending.set_value(None);
            if !mounted.get_untracked() || !root.is_connected() {
                return;
            }
            let origin = root.get_bounding_client_rect();
            let bounds = elements
                .iter()
                .map(|(key, element)| (*key, element.get_bounding_client_rect()))
                .collect::<HashMap<_, _>>();
            let routed = edges
                .iter()
                .map(|edge| {
                    let from = bounds.get(&(edge.from, edge.source_row))?;
                    let to = bounds.get(&(edge.to, edge.target_row))?;
                    Some((
                        (
                            from.right() - origin.left(),
                            from.top() + from.height() / 2.0 - origin.top(),
                        ),
                        (
                            to.left() - origin.left(),
                            to.top() + to.height() / 2.0 - origin.top(),
                        ),
                    ))
                })
                .collect::<Vec<_>>();
            let mut lane_counts = HashMap::<(i64, i64), usize>::new();
            for (from, to) in routed.iter().flatten() {
                *lane_counts
                    .entry((from.0.round() as i64, to.0.round() as i64))
                    .or_default() += 1;
            }
            let mut lane_indices = HashMap::<(i64, i64), usize>::new();
            let drawn = routed
                .into_iter()
                .map(|route| {
                    let Some((from, to)) = route else {
                        return String::new();
                    };
                    let key = (from.0.round() as i64, to.0.round() as i64);
                    let lane = lane_indices.entry(key).or_default();
                    let fraction = (*lane + 1) as f64 / (lane_counts[&key] + 1) as f64;
                    let bend = if to.0 > from.0 {
                        from.0 + (to.0 - from.0) * fraction
                    } else {
                        from.0 + 12.0 + *lane as f64 * 8.0
                    };
                    *lane += 1;
                    format!("M {} {} H {bend} V {} H {}", from.0, from.1, to.1, to.0)
                })
                .collect::<Vec<_>>();
            if paths.with_untracked(|current| *current != drawn) {
                paths.set(drawn);
            }
        });
        pending.set_value(handle.ok());
    });

    view! {
        <svg
            class="absolute inset-0 z-0 w-full h-full pointer-events-none"
            aria-hidden="true"
            fill="none"
        >
            {edges
                .into_iter()
                .enumerate()
                .map(|(index, edge)| {
                    let paths = paths.clone();
                    let editing = editing.clone();
                    let class = move || {
                        let active_player = editing
                            .as_ref()
                            .and_then(|editor| editor.active_player.get())
                            .or_else(|| highlighted.get())
                            .or_else(|| selection.get().map(TournamentSelection::player));
                        if active_player
                            .is_some_and(|player| {
                                edge.routed_player == Some(player)
                                    || editing
                                        .as_ref()
                                        .is_some_and(|editor| {
                                            editor
                                                .possible
                                                .with(|possible| {
                                                    possible
                                                        .get(&(edge.to, edge.target_row))
                                                        .is_some_and(|candidates| candidates.contains(&player))
                                                })
                                        })
                            })
                        {
                            "stroke-pillbug-teal"
                        } else if active_player.is_some() {
                            "stroke-gray-300 opacity-30 dark:stroke-gray-700"
                        } else {
                            "stroke-gray-400 dark:stroke-gray-600"
                        }
                    };
                    view! {
                        <path
                            d=move || paths.with(|paths| paths[index].clone())
                            stroke-dasharray=if edge.kind == RouteKind::Winner { "" } else { "4 4" }
                            class=class
                            stroke-width="1.5"
                        />
                    }
                })
                .collect_view()}
        </svg>
    }
}
