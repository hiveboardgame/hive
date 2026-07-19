use crate::providers::{
    analysis::{AnalysisSignal, AnalysisTree, TreeNode},
    game_state::GameStateStore,
};
use leptos::prelude::*;

#[component]
pub fn VariationList(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    let alternate_moves = Memo::new(move |_| analysis.tree.with(alternate_moves_for));
    let list_class = move || {
        format!(
            "w-fit max-w-full {} {}",
            extend_tw_classes,
            if alternate_moves.with(|moves| moves.is_empty()) {
                "hidden"
            } else {
                ""
            },
        )
    };
    let moves = move || {
        alternate_moves.with(|moves| {
            moves
                .iter()
                .map(|(node_id, TreeNode {
                    turn,
                    piece,
                    position,
                })| {
                    let node_id = *node_id;
                    let move_text = format!("{turn}. {piece} {position}");
                    view! {
                        <div
                            class="flex items-center px-2 h-6 font-mono text-xs whitespace-nowrap rounded transition-colors cursor-pointer active:scale-95 dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70"
                            on:click=move |_| {
                                analysis
                                    .tree
                                    .update(|a| {
                                        a.update_node(node_id, Some(game_state));
                                    });
                                analysis.sync_reserve_from_game_state(game_state);
                            }
                        >
                            {move_text}
                        </div>
                    }
                })
                .collect_view()
        })
    };
    view! {
        <div class=list_class>
            <div class="inline-flex flex-wrap gap-2 items-center py-1 px-3 max-w-full text-sm rounded border shadow-sm min-h-10 border-black/10 bg-even-light/90 backdrop-blur dark:border-white/10 dark:bg-surface-panel/90">
                <span class="text-xs font-semibold text-gray-600 uppercase whitespace-nowrap dark:text-gray-300 shrink-0">
                    "Other lines"
                </span>
                {moves}
            </div>
        </div>
    }
}

fn alternate_moves_for(analysis: &AnalysisTree) -> Vec<(i32, TreeNode)> {
    let tree = &analysis.tree;
    analysis
        .current_node
        .as_ref()
        .and_then(|node| node.get_node_id().ok())
        .and_then(|node_id| tree.get_sibling_ids(&node_id, false).ok())
        .map_or_else(Vec::new, |sibling_ids| {
            sibling_ids
                .iter()
                .filter_map(|id| {
                    tree.get_node_by_id(id)
                        .and_then(|node| node.get_value().ok().flatten())
                        .map(|tree_node| (*id, tree_node))
                })
                .collect()
        })
}
