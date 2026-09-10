use super::{
    is_double_elimination,
    pending_entrant_label,
    semantic_phase_count,
    semantic_phase_index,
    semantic_phase_label,
    BracketEditingContext,
    BracketModel,
    BracketPosition,
    NodeState,
};
use crate::i18n::*;
use leptos::{
    ev::scroll,
    html::{Div, Input},
    leptos_dom::helpers::request_animation_frame,
    prelude::*,
};
use leptos_use::{use_event_listener, use_media_query, use_window_size};
use shared_types::tournament::elimination::{Resolution, Source};
use std::collections::HashSet;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[derive(Clone, PartialEq)]
struct PlayerChoice {
    id: Uuid,
    name: String,
    seed: usize,
    detail: String,
}

pub(super) fn assign_player(
    order: &mut [Uuid],
    model: &BracketModel,
    position: BracketPosition,
    player: Uuid,
) {
    let Some(node) = model.nodes.iter().find(|node| node.id == position.node) else {
        return;
    };
    if !matches!(node.state, NodeState::Active | NodeState::Planned { .. }) {
        return;
    }
    let Some(Some(occupant)) = node.entrants.get(position.index) else {
        return;
    };
    if let (Some(destination), Some(source)) = (
        order.iter().position(|id| id == occupant),
        order.iter().position(|id| *id == player),
    ) {
        order.swap(destination, source);
    }
}

fn position_button(root: NodeRef<Div>, position: BracketPosition) -> Option<HtmlElement> {
    root.get_untracked()?
        .query_selector(&format!(
            "[data-elimination-node=\"{}\"] [data-entrant-index=\"{}\"] [data-bracket-position]",
            position.node.value(),
            position.index,
        ))
        .ok()??
        .dyn_into()
        .ok()
}

fn close_picker(root: NodeRef<Div>, position: RwSignal<Option<BracketPosition>>) {
    let target = position.get_untracked();
    position.set(None);
    let Some(target) = target else { return };
    let Some(root) = root.get_untracked() else {
        return;
    };
    let position: ArcRwSignal<Option<BracketPosition>> = position.into();
    // Assignment can rebuild the graph. Restore focus to the current DOM after that
    // rebuild, provided the review is still mounted and no new assignment has begun.
    request_animation_frame(move || {
        if !root.is_connected() || position.get_untracked().is_some() {
            return;
        }
        if let Some(button) = root
            .query_selector(&format!(
            "[data-elimination-node=\"{}\"] [data-entrant-index=\"{}\"] [data-bracket-position]",
            target.node.value(), target.index,
        ))
            .ok()
            .flatten()
            .and_then(|button| button.dyn_into::<HtmlElement>().ok())
        {
            let _ = button.focus();
        }
    });
}

#[component]
pub(super) fn PlacementPicker(editor: BracketEditingContext, root: NodeRef<Div>) -> impl IntoView {
    let i18n = use_i18n();
    let panel = NodeRef::<Div>::new();
    let input = NodeRef::<Input>::new();
    let query = RwSignal::new(String::new());
    let style = RwSignal::new(String::new());
    let position = editor.position;
    let preview = editor.preview;
    let names = editor.names;
    let order = editor.order;
    let window_size = use_window_size();
    let has_fine_pointer = use_media_query("(pointer: fine)");
    let close = Callback::new(move |()| close_picker(root, position));
    let scroll_area = Signal::derive(move || {
        root.get().and_then(|root| {
            root.closest("[data-testid=\"bracket-editor-scroll\"]")
                .ok()
                .flatten()
        })
    });
    let _ = use_event_listener(scroll_area, scroll, move |_| position.set(None));
    let choices = Memo::new(move |_| {
        let names = names.get();
        preview.with(|preview| {
            let Some((model, _)) = preview else { return Vec::new() };
            let phase_count = semantic_phase_count(&model.nodes, is_double_elimination(&model.nodes));
            editor.seeded_players.iter().enumerate().filter_map(|(seed, id)| {
                let node = model.nodes.iter().find(|node| {
                    matches!(node.state, NodeState::Active | NodeState::Planned { .. })
                        && node.entrants.contains(&Some(*id))
                })?;
                let index = node.entrants.iter().position(|entrant| *entrant == Some(*id))?;
                let opponent = node.entrants[1 - index]
                    .and_then(|id| names.get(&id).cloned())
                    .unwrap_or_else(|| pending_entrant_label(
                        i18n, node.sources[1 - index], &model.nodes, &HashSet::new(),
                    ));
                let phase = semantic_phase_label(
                    phase_count, semantic_phase_index(node.stage, phase_count).unwrap_or(0),
                );
                let bye = matches!(node.sources[index], Source::WinnerOf { node_id }
                    if model.nodes.iter().any(|source| source.id == node_id
                        && matches!(source.state, NodeState::Resolved(Resolution::AutomaticAdvance { .. }))));
                // TODO: i18n once copy is approved.
                Some(PlayerChoice {
                    id: *id,
                    name: names.get(id).cloned().unwrap_or_default(),
                    seed: seed + 1,
                    detail: format!("{phase} · Match {} · {}vs {opponent}", node.stage_ordinal + 1, if bye { "Bye · " } else { "" }),
                })
            }).collect::<Vec<_>>()
        })
    });
    let results = Memo::new(move |_| {
        let query = query.get().trim().to_lowercase();
        choices.with(|choices| {
            choices
                .iter()
                .filter(|choice| {
                    let searchable = format!("{} #{}", choice.name, choice.seed).to_lowercase();
                    query
                        .split_whitespace()
                        .all(|word| searchable.contains(word))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let title = move || {
        let target = position.get()?;
        preview.with(|preview| {
            let model = &preview.as_ref()?.0;
            let node = model.nodes.iter().find(|node| node.id == target.node)?;
            // TODO: i18n once copy is approved.
            Some(format!(
                "Match {} · Player {}",
                node.stage_ordinal + 1,
                target.index + 1
            ))
        })
    };
    let choose = Callback::new(move |player: Uuid| {
        let Some(target) = position.get_untracked() else {
            return;
        };
        let Some((model, _)) = preview.get_untracked() else {
            return;
        };
        let Some(node) = model.nodes.iter().find(|node| node.id == target.node) else {
            return;
        };
        order.update(|order| assign_player(order, &model, target, player));
        let next = (target.index == 0 && node.entrants[1].is_some()).then_some(BracketPosition {
            node: target.node,
            index: 1,
        });
        query.set(String::new());
        if next.is_some() {
            position.set(next);
        } else {
            close.run(());
        }
    });

    Effect::new(move |_| {
        let Some(target) = position.get() else { return };
        let Some(_) = panel.get() else { return };
        let Some(button) = position_button(root, target) else {
            return;
        };
        let anchor = button
            .closest("article")
            .ok()
            .flatten()
            .map(|card| card.get_bounding_client_rect())
            .unwrap_or_else(|| button.get_bounding_client_rect());
        let width = window_size.width.get();
        let height = window_size.height.get();
        let scroll_bounds = scroll_area
            .get()
            .map(|area| area.get_bounding_client_rect());
        let top_limit = scroll_bounds
            .as_ref()
            .map(|rect| rect.top() + 8.0)
            .unwrap_or(12.0)
            .max(12.0);
        let bottom_limit = scroll_bounds
            .as_ref()
            .map(|rect| rect.bottom() - 8.0)
            .unwrap_or(height - 12.0)
            .min(height - 12.0);
        let below = (bottom_limit - anchor.bottom() - 6.0).max(0.0);
        let above = (anchor.top() - 6.0 - top_limit).max(0.0);
        let place_below = below >= 240.0 || below >= above;
        let panel_width = 352.0_f64.min((width - 24.0).max(0.0));
        let panel_height = 384.0_f64.min(if place_below { below } else { above });
        let left = anchor.left().min(width - panel_width - 12.0).max(12.0);
        let top = if place_below {
            anchor.bottom() + 6.0
        } else {
            (anchor.top() - panel_height - 6.0).max(top_limit)
        };
        style.set(format!(
            "left:{left}px;top:{top}px;width:{panel_width}px;max-height:{panel_height}px"
        ));
    });

    Effect::new(move |_| {
        let Some(_) = position.get() else { return };
        let Some(panel) = panel.get() else { return };
        let Some(input) = input.get() else { return };
        let _ = panel.show_popover();
        if has_fine_pointer.get_untracked() {
            let _ = input.focus();
        } else {
            // Touch users should opt into the keyboard by tapping search.
            let _ = panel.focus();
        }
    });

    // TODO: i18n once copy is approved.
    view! {
        <div
            node_ref=panel
            id="bracket-player-picker"
            popover="auto"
            role="dialog"
            tabindex="-1"
            aria-label="Assign player"
            class="fixed flex-col gap-2 p-3 m-0 text-gray-900 rounded-lg border border-gray-300 shadow-xl dark:text-gray-100 dark:border-gray-600 bg-even-light open:flex dark:bg-surface-panel"
            style=move || style.get()
            on:toggle=move |_| {
                if panel
                    .get_untracked()
                    .is_some_and(|panel| !panel.matches(":popover-open").unwrap_or(false))
                {
                    position.set(None);
                }
            }
            on:keydown=move |event| {
                if event.key() == "Escape" {
                    event.prevent_default();
                    event.stop_propagation();
                    close.run(());
                }
            }
        >
            <div class="flex gap-2 justify-between items-center mb-2 shrink-0">
                <p class="text-sm font-semibold">{title}</p>
                <button
                    type="button"
                    class="ui-button ui-button-ghost ui-button-icon-sm"
                    aria-label="Close player picker"
                    on:click=move |_| close.run(())
                >
                    "×"
                </button>
            </div>
            <input
                node_ref=input
                type="search"
                placeholder="Search players…"
                aria-label="Search players"
                autocomplete="off"
                class="py-2 px-3 mb-2 w-full text-sm bg-white rounded border border-gray-300 dark:border-gray-600 shrink-0 dark:bg-surface-raised"
                prop:value=move || query.get()
                on:input=move |event| query.set(event_target_value(&event))
                on:keydown=move |event| {
                    if event.key() == "Enter" {
                        event.prevent_default();
                        if let Some(first) = results
                            .with_untracked(|results| results.first().map(|choice| choice.id))
                        {
                            choose.run(first);
                        }
                    } else if event.key() == "ArrowDown" {
                        event.prevent_default();
                        if let Some(first) = panel
                            .get_untracked()
                            .and_then(|panel| {
                                panel.query_selector("[data-player-choice]").ok().flatten()
                            })
                            .and_then(|first| first.dyn_into::<HtmlElement>().ok())
                        {
                            let _ = first.focus();
                        }
                    }
                }
            />
            <div class="overflow-y-auto min-h-0 max-h-64">
                <For
                    each=move || results.get()
                    key=|choice| (choice.id, choice.name.clone(), choice.detail.clone())
                    let:choice
                >
                    <button
                        type="button"
                        data-player-choice=choice.id.to_string()
                        class="block py-2 px-2 w-full text-left rounded hover:bg-pillbug-teal/10 focus-visible:bg-pillbug-teal/10 focus-visible:outline-2 focus-visible:outline-pillbug-teal"
                        on:click=move |_| choose.run(choice.id)
                    >
                        <span class="flex gap-2 items-baseline text-sm">
                            <span class="text-xs tabular-nums text-gray-500">
                                {format!("#{}", choice.seed)}
                            </span>
                            <span class="font-semibold break-all">{choice.name}</span>
                        </span>
                        <span class="block text-xs text-gray-500 dark:text-gray-400">
                            {choice.detail}
                        </span>
                    </button>
                </For>
                <Show when=move || results.with(Vec::is_empty)>
                    <p class="p-2 text-sm text-gray-500">"No players found."</p>
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::organisms::bracket::placement_model;
    use tournamint::elimination::EliminationKind;

    #[test]
    fn assignment_uses_visible_positions_including_collapsed_byes() {
        let players = (1..=33).map(Uuid::from_u128).collect::<Vec<_>>();
        for kind in [
            EliminationKind::Single { bronze: true },
            EliminationKind::Double,
        ] {
            let (model, _) = placement_model(kind, &players, &players).unwrap();
            let positions = model
                .nodes
                .iter()
                .filter(|node| matches!(node.state, NodeState::Active | NodeState::Planned { .. }))
                .flat_map(|node| {
                    node.entrants
                        .iter()
                        .enumerate()
                        .filter_map(move |(index, player)| {
                            player.map(|player| {
                                (
                                    BracketPosition {
                                        node: node.id,
                                        index,
                                    },
                                    player,
                                )
                            })
                        })
                })
                .collect::<Vec<_>>();
            for &(destination, occupant) in &positions {
                let &(source, incoming) = positions
                    .iter()
                    .find(|(_, player)| *player != occupant)
                    .unwrap();
                let mut order = players.clone();
                assign_player(&mut order, &model, destination, incoming);
                let (updated, _) = placement_model(kind, &players, &order).unwrap();
                let occupant_at = |position: BracketPosition| {
                    updated
                        .nodes
                        .iter()
                        .find(|node| node.id == position.node)
                        .unwrap()
                        .entrants[position.index]
                };
                assert_eq!(occupant_at(destination), Some(incoming));
                assert_eq!(occupant_at(source), Some(occupant));
                assert_eq!(
                    order.iter().copied().collect::<HashSet<_>>(),
                    players.iter().copied().collect()
                );
            }
        }
    }
}
