use crate::{
    common::{CurrentConfirm, MoveConfirm},
    components::{
        layouts::base_layout::OrientationSignal,
        organisms::{
            analysis::{History, HistoryButton, HistoryNavigation, OpeningExplorer, UndoButton},
            board::Board,
            reserve::{Alignment, Reserve},
        },
    },
    functions::games::get::get_game_from_nanoid,
    hiveground::{analysis_hiveground_interaction, selected_history_state, HivegroundInteraction},
    providers::{
        analysis::{AnalysisSignal, AnalysisTree, TreeNode},
        annotations::AnnotationsSignal,
        game_state::GameStateSignal,
        AuthContext,
    },
    responses::GameResponse,
};
use hive_lib::{Color, GameStatus, GameType, State};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{GameId, TimeMode};
use std::collections::HashSet;

#[derive(Clone)]
pub struct ToggleStates(pub RwSignal<HashSet<i32>>);

#[component]
pub fn Analysis(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let history_state = selected_history_state(game_state);
    let params = use_params_map();
    let queries = use_query_map();
    let game_id = Memo::new(move |_| params().get("nanoid").map(|s| GameId(s.to_owned())));
    let move_number = StoredValue::new(
        queries
            .get_untracked()
            .get("move")
            .and_then(|s| s.parse::<usize>().ok())
            .map(|n| n.saturating_sub(1)),
    );
    let uhp_string = StoredValue::new(queries.get_untracked().get("uhp"));

    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;

    let should_block_analysis = move |game_response: &GameResponse| -> bool {
        let Some(user_id) = auth_context.user.with(|u| u.as_ref().map(|u| u.id)) else {
            return false;
        };

        game_response.rated
            && matches!(game_response.game_status, GameStatus::InProgress)
            && game_response.time_mode == TimeMode::RealTime
            && (Some(user_id) == Some(game_response.white_player.uid)
                || Some(user_id) == Some(game_response.black_player.uid))
    };

    let game_resource = Resource::new(game_id, move |game_id| async move {
        if let Some(game_id) = game_id {
            get_game_from_nanoid(game_id).await
        } else {
            Err(leptos::prelude::ServerFnError::new("No game ID provided"))
        }
    });

    view! {
        <div class=move || {
            format!(
                "pt-10 bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                if vertical() {
                    "flex flex-col h-full standalone:min-h-[var(--app-height)]"
                } else {
                    "max-h-[100dvh] min-h-[100dvh] standalone:max-h-[var(--app-height)] standalone:min-h-[var(--app-height)] grid grid-cols-10  grid-rows-6 pr-1"
                },
            )
        }>
            <Suspense fallback=move || {
                view! { <div>"Loading analysis..."</div> }
            }>
                {move || {
                    let analysis_signal = game_resource
                        .with(|gr| {
                            let analysis_tree = match gr {
                                Some(Ok(game_response)) if !should_block_analysis(game_response) => {
                                    AnalysisTree::from_game_response(
                                            game_response,
                                            game_state,
                                            move_number.get_value(),
                                        )
                                        .unwrap_or_default()
                                }
                                _ => {
                                    uhp_string
                                        .get_value()
                                        .and_then(|uhp| {
                                            AnalysisTree::from_uhp(game_state, uhp).ok()
                                        })
                                        .unwrap_or_else(|| {
                                            AnalysisTree::new_blank_analysis(game_state, GameType::MLP)
                                        })
                                }
                            };
                            AnalysisSignal(RwSignal::new(analysis_tree))
                        });
                    provide_context(analysis_signal);
                    provide_context(AnnotationsSignal::analysis(analysis_signal));
                    let hiveground_interaction = analysis_hiveground_interaction();

                    view! {
                        <Show
                            when=vertical
                            fallback=move || {
                                view! {
                                    <AnalysisInfo extend_tw_classes="absolute pl-4 pt-2 bg-transparent" />
                                    <Board interaction=hiveground_interaction history_state />
                                    <AnalysisSidebar
                                        interaction=hiveground_interaction
                                        history_state
                                    />
                                }
                            }
                        >
                            <div class="flex flex-col h-[85dvh]">
                                <div class="flex flex-col flex-grow shrink">
                                    <div class="flex justify-between h-full max-h-16">
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::White
                                            interaction=hiveground_interaction
                                            history_state
                                        />
                                    </div>
                                </div>
                                <AnalysisInfo extend_tw_classes="border-gray-500 border-dashed border-b-[1px]" />
                                <Board interaction=hiveground_interaction history_state />
                                <div class="border-gray-500 border-dashed border-t-[1px]"></div>
                                <div class="flex flex-col flex-grow shrink">
                                    <div class="flex justify-between h-full max-h-16">
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::Black
                                            interaction=hiveground_interaction
                                            history_state
                                        />
                                    </div>
                                </div>
                            </div>
                            <History mobile=true interaction=hiveground_interaction history_state />
                            <OpeningExplorer />
                        </Show>
                    }
                }}
            </Suspense>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq)]
enum AnalysisTab {
    History,
    Explorer,
}

/// The desktop analysis sidebar: History and Opening Explorer as tabs, so the explorer gets the
/// full sidebar height instead of sharing it with the move history. The reserve is unaffected
/// (it lives in the board on desktop / as strips on mobile).
#[component]
fn AnalysisSidebar(
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
) -> impl IntoView {
    let tab = RwSignal::new(AnalysisTab::History);
    let trigger_class = move |name: AnalysisTab| {
        move || {
            format!(
                "grow py-1 text-center cursor-pointer transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal {}",
                if tab() == name {
                    "dark:bg-button-twilight bg-slate-400"
                } else {
                    "bg-inherit"
                },
            )
        }
    };
    view! {
        <div class="flex flex-col col-span-2 row-span-6 h-full border-2 border-black select-none dark:border-white">
            <div class="flex sticky top-0 z-10 justify-between border-b-2 border-black dark:border-white">
                <div
                    class=trigger_class(AnalysisTab::History)
                    on:click=move |_| tab.set(AnalysisTab::History)
                >
                    "History"
                </div>
                <div
                    class=trigger_class(AnalysisTab::Explorer)
                    on:click=move |_| tab.set(AnalysisTab::Explorer)
                >
                    "Explorer"
                </div>
            </div>
            <div class="overflow-y-auto flex-grow p-1">
                <Show when=move || tab() == AnalysisTab::History>
                    <History interaction history_state />
                </Show>
                <Show when=move || tab() == AnalysisTab::Explorer>
                    <div class="flex gap-1 min-h-0 [&>*]:grow">
                        <HistoryButton action=HistoryNavigation::First post_action=None />
                        <HistoryButton action=HistoryNavigation::Previous post_action=None />
                        <HistoryButton action=HistoryNavigation::Next post_action=None />
                        <UndoButton />
                    </div>
                    <div class="flex flex-col px-4 pt-2">
                        <Reserve
                            alignment=Alignment::DoubleRow
                            color=Color::White
                            viewbox_str="-32 -40 250 120"
                            interaction
                            history_state
                        />
                        <Reserve
                            alignment=Alignment::DoubleRow
                            color=Color::Black
                            viewbox_str="-32 -40 250 120"
                            interaction
                            history_state
                        />
                    </div>
                    <OpeningExplorer />
                </Show>
            </div>
        </div>
    }
}

#[component]
fn AnalysisInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let moves = move || {
        analysis.with(|a| {
            let tree = &a.tree;
            let mut moves = Vec::new();
            let sibling_nodes = a.current_node
                .as_ref()
                .and_then(|n| n.get_node_id().ok())
                .and_then(|node_id| tree.get_sibling_ids(&node_id, false).ok())
                .map_or(Vec::new(), |s| {
                    s.iter()
                        .filter_map(|id| tree.get_node_by_id(id))
                        .collect::<Vec<_>>()
                });
            for s in sibling_nodes {
                if let Ok(Some(TreeNode {
                    turn,
                    piece,
                    position,
                })) = s.get_value() {
                moves.push(
                        view! {
                            <div
                                class="underline cursor-pointer active:scale-95 no-link-style hover:text-pillbug-teal"
                                on:click=move |_| {
                                    analysis
                                        .update(|a| {
                                            if let Ok(node_id) = s.get_node_id() {
                                                a.update_node(node_id, Some(game_state));
                                            }
                                        })
                                }
                            >
                                {format!("{turn}. {piece} {position}")}
                            </div>
                        }
                    );
                }
            }
            moves.collect_view()
        })
    };
    view! {
        <div class=extend_tw_classes>
            <div class="flex gap-1 items-center">
                <b>"Other Lines Explored: "</b>
                {moves}
            </div>
        </div>
    }
}
