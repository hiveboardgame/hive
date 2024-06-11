use super::{History, TreeNode};
use crate::components::organisms::reserve::Alignment;
use crate::components::organisms::reserve::Reserve;
use crate::{
    components::organisms::analysis::{AnalysisSignal, ToggleStates},
    providers::game_state::GameStateSignal,
};
use hive_lib::Color;
use leptix_primitives::components::{
    collapsible::{CollapsibleContent, CollapsibleRoot, CollapsibleTrigger},
    tabs::{TabsContent, TabsList, TabsRoot, TabsTrigger},
};
use leptos::*;
use leptos_icons::Icon;
use tree_ds::prelude::Node;
#[component]
pub fn SideboardTabs(
    player_color: Memo<Color>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let button_class = move || {
        "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal data-[state=active]:dark:bg-button-twilight data-[state=active]:bg-slate-400".to_string()
    };
    view! {
        <TabsRoot
            default_value="Game"
            attr:class=format!(
                "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-1 {extend_tw_classes}",
            )
        >

            <TabsList>
                <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">
                    <TabsTrigger value="Game" attr:class=button_class>
                        "Game"
                    </TabsTrigger>
                    <TabsTrigger value="History" attr:class=button_class>
                        "History"
                    </TabsTrigger>
                </div>
            </TabsList>
            <TabsContent value="Game" attr:class="flex flex-col h-full">
                <ReserveContent player_color/>
            </TabsContent>
            <TabsContent
                value="History"
                attr:class="overflow-auto gap-1 mb-8 w-full h-full max-h-full h-fit"
            >
                <History/>
            </TabsContent>
        </TabsRoot>
    }
}

#[component]
pub fn UndoButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let undo = move |_| {
        analysis.update(|a| {
            if let Some(a) = a {
                if let Some(node) = a.current_node.clone() {
                    let new_current = node.get_parent_id();
                    if let Some(new_current) = new_current {
                        a.update_node(new_current);
                        a.tree
                            .remove_node(
                                &node.get_node_id(),
                                tree_ds::prelude::NodeRemovalStrategy::RemoveNodeAndChildren,
                            )
                            .unwrap();
                    } else {
                        a.reset();
                    }
                }
            }
        });
    };

    view! {
        <button
            class="flex justify-center items-center rounded-sm transition-transform duration-300 transform aspect-square hover:bg-pillbug-teal active:scale-95"
            on:click=undo
        >
            <Icon icon=icondata::BiUndoRegular class="w-6 h-6 lg:h-8 lg:w-8"/>
        </button>
    }
}

#[component]
pub fn ReserveContent(player_color: Memo<Color>) -> impl IntoView {
    let top_color = Signal::derive(move || player_color().opposite_color());
    let bottom_color = Signal::derive(player_color);
    view! {
        <Reserve color=top_color alignment=Alignment::DoubleRow/>
        <div class="flex flex-row-reverse justify-center items-center">
            <UndoButton/>
        </div>
        <Reserve color=bottom_color alignment=Alignment::DoubleRow/>
    }
}

use leptos::leptos_dom::helpers::debounce;
#[derive(Clone)]
pub enum HistoryNavigation {
    Next,
    Previous,
}

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    #[prop(optional)] post_action: Option<Callback<()>>,
    #[prop(optional)] node_ref: Option<NodeRef<html::Button>>,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::Next => icondata::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata::AiStepBackwardFilled,
    };

    let is_disabled = move || {
        let analysis = analysis.get().unwrap();
        analysis.current_node.map_or(true, |n| match cloned_action {
            HistoryNavigation::Next => n.get_children_ids().is_empty(),
            HistoryNavigation::Previous => n.get_parent_id().is_none(),
        })
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        let current_node = analysis.get().unwrap().current_node;
        let updated_node_id = current_node.and_then(|n| match action {
            HistoryNavigation::Next => n.get_children_ids().first().cloned(),
            HistoryNavigation::Previous => n.get_parent_id(),
        });
        if let Some(updated_node_id) = updated_node_id {
            analysis.update(|a| {
                if let Some(a) = a {
                    a.update_node(updated_node_id);
                }
            });
        }
        if let Some(post_action) = post_action {
            post_action(())
        }
    });
    let _definite_node_ref = node_ref.unwrap_or(create_node_ref::<html::Button>());

    view! {
        <button
            ref=_definite_node_ref
            class=nav_buttons_style
            prop:disabled=is_disabled
            on:click=debounced_action
        >

            <Icon icon=icon/>
        </button>
    }
}

#[component]
pub fn HistoryMove(node: Node<i32, TreeNode>) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let value = node.get_value().unwrap();
    let node_id = node.get_node_id();
    let class = move || {
        let mut class =
            "w-full transition-transform duration-300 transform hover:bg-pillbug-teal active:scale-95";
        if analysis
            .get()
            .unwrap()
            .current_node
            .map_or(false, |n| n.get_node_id() == node_id)
        {
            class = "w-full transition-transform duration-300 transform hover:bg-pillbug-teal bg-orange-twilight active:scale-95"
        }
        class
    };
    let onclick = move |_| {
        analysis.update(|a| {
            if let Some(a) = a {
                a.update_node(node_id);
            }
        });
    };
    view! {
        <div class=class on:click=onclick>
            {format!("{}. {} {}", value.turn, value.piece, value.position)}
        </div>
    }
}

#[component]
pub fn CollapsibleMove(node: Node<i32, TreeNode>, children: ChildrenFn) -> impl IntoView {
    let closed_toggles = expect_context::<ToggleStates>().0;
    let node_id = node.get_node_id();
    let is_open = !closed_toggles.get().contains(&node_id);
    let (open, set_open) = create_signal(is_open);
    view! {
        <CollapsibleRoot
            open
            on_open_change=move |s: bool| {
                closed_toggles
                    .update_untracked(|t| {
                        if s {
                            t.remove(&node_id);
                        } else {
                            t.insert(node_id);
                        }
                    });
                set_open(s);
            }
        >

            <div class="flex">
                <CollapsibleTrigger>
                    <button>
                        <svg
                            width="15"
                            height="15"
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points=move || {
                                if open() { "19 12 12 19 5 12" } else { "12 5 19 12 12 19" }
                            }></polyline>
                        </svg>
                    </button>
                </CollapsibleTrigger>
                <HistoryMove node=node.clone()/>
            </div>
            <CollapsibleContent children=children.clone() attr:class="nested-content"/>
        </CollapsibleRoot>
    }
}
