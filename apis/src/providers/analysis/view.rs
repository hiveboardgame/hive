use super::tree::{AnalysisArena, MoveDelta, NodeId};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibleRow {
    pub node_id: NodeId,
    pub indent: usize,
    pub has_variations: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchSummary {
    pub node_id: NodeId,
    pub move_delta: MoveDelta,
    pub node_count: usize,
}

fn path_contains(arena: &AnalysisArena, path: &[NodeId], node_id: NodeId) -> bool {
    arena
        .node(node_id)
        .is_some_and(|node| path.get(node.depth).copied() == Some(node_id))
}

pub(super) fn variation_is_forced_open(
    arena: &AnalysisArena,
    node_id: NodeId,
    selected_path: &[NodeId],
) -> bool {
    arena.node(node_id).is_some_and(|node| {
        node.children
            .iter()
            .skip(1)
            .any(|child| path_contains(arena, selected_path, *child))
    })
}

pub(super) fn build_visible_rows(
    arena: &AnalysisArena,
    collapsed: &HashSet<NodeId>,
    selected_path: &[NodeId],
) -> Vec<VisibleRow> {
    let Some(root) = arena.node(arena.root) else {
        return Vec::new();
    };
    let show_root = root.children.len() > 1;
    let mut rows = Vec::with_capacity(arena.nodes.len().saturating_sub(!show_root as usize));
    let mut stack = Vec::new();
    if show_root {
        stack.push((arena.root, 0));
    } else if let Some(child) = root.children.first() {
        stack.push((*child, 0));
    }
    while let Some((id, indent)) = stack.pop() {
        let Some(node) = arena.node(id) else {
            continue;
        };
        let variations_open =
            !collapsed.contains(&id) || variation_is_forced_open(arena, id, selected_path);
        rows.push(VisibleRow {
            node_id: id,
            indent,
            has_variations: node.children.len() > 1,
        });
        if let Some(main) = node.children.first() {
            stack.push((*main, indent));
        }
        if variations_open {
            for child in node.children.iter().skip(1).rev() {
                stack.push((*child, indent + 1));
            }
        }
    }
    rows
}
