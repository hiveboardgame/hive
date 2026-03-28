use crate::{
    common::MoveConfirm,
    components::{
        layouts::base_layout::OrientationSignal,
        organisms::{
            analysis::History,
            board::Board,
            reserve::{Alignment, Reserve},
        },
    },
    functions::games::get::get_game_from_nanoid,
    pages::play::CurrentConfirm,
    providers::{
        analysis::{AnalysisStore, AnalysisTree, AnalysisTreeStoreFields, TreeNode},
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
    },
    responses::GameResponse,
};
use hive_lib::{Color, GameError, GameStatus, GameType, History, State};
use leptos::{either::Either, prelude::*};
use leptos_router::hooks::{use_params_map, use_query_map};
use reactive_stores::Store;
use shared_types::{GameId, TimeMode};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone)]
pub struct ToggleStates(pub RwSignal<HashSet<i32>>);

fn should_block_analysis(user_id: Option<Uuid>, game_response: &GameResponse) -> bool {
    user_id.is_some_and(|user_id| {
        game_response.rated
            && matches!(game_response.game_status, GameStatus::InProgress)
            && game_response.time_mode == TimeMode::RealTime
            && (Some(user_id) == Some(game_response.white_player.uid)
                || Some(user_id) == Some(game_response.black_player.uid))
    })
}

fn state_from_uhp(uhp_string: impl Into<String>) -> Result<State, GameError> {
    let history = match History::from_uhp_str(uhp_string.into()) {
        Ok(history) => history,
        Err(GameError::PartialHistory { history, .. }) => history,
        Err(err) => return Err(err),
    };
    State::new_from_history(&history)
}

fn analysis_tree_from_sources(
    state_from_uhp: Option<State>,
    game_response: Option<&GameResponse>,
    move_number: Option<usize>,
) -> AnalysisTree {
    let fallback_game_type = game_response
        .map(|game_response| game_response.game_type)
        .unwrap_or(GameType::MLP);

    let analysis_tree = state_from_uhp
        .map(|state| AnalysisTree::from_loaded_state(&state, move_number))
        .or_else(|| {
            game_response
                .map(|game_response| AnalysisTree::from_game_response(game_response, move_number))
        })
        .unwrap_or_else(|| AnalysisTree::new_blank_analysis(fallback_game_type));

    analysis_tree
}

#[component]
pub fn Analysis(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let params = use_params_map();
    let queries = use_query_map();
    let game_id = Memo::new(move |_| params.get().get("nanoid").map(|s| GameId(s.to_owned())));
    let query_inputs = Memo::new(move |_| {
        let q = queries.get();
        let move_number = q
            .get("move")
            .and_then(|s| s.parse::<usize>().ok())
            .map(|n| n.saturating_sub(1));
        let uhp_string = q.get("uhp");
        (move_number, uhp_string)
    });
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;

    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let game_resource = LocalResource::new(move || {
        let game_id = game_id.get();
        async move {
            if let Some(game_id) = game_id {
                let result = get_game_from_nanoid(game_id.clone()).await;
                result
            } else {
                Err(leptos::prelude::ServerFnError::new("No game ID provided"))
            }
        }
    });

    view! {
        <div class=move || {
            format!(
                "pt-10 bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                if vertical() {
                    "flex flex-col h-full"
                } else {
                    "max-h-[100dvh] min-h-[100dvh] grid grid-cols-10  grid-rows-6 pr-1"
                },
            )
        }>
            <Suspense fallback=move || {
                view! { <div>"Loading analysis..."</div> }
            }>
                {move || {
                    game_resource
                        .get()
                        .map(|result| {
                            let game_response = result.ok();
                            let user_id = auth_context.user.with(|u| u.as_ref().map(|u| u.id));
                            let should_block_analysis = game_response
                                .as_ref()
                                .is_some_and(|game_response| {
                                    should_block_analysis(user_id, game_response)
                                });
                            if should_block_analysis {
                                Either::Left(

                                    view! { <div>"Analysis is unavailable for this game."</div> },
                                )
                            } else {
                                let (move_number, uhp_string) = query_inputs.get();
                                let uhp_state = uhp_string.and_then(|uhp| state_from_uhp(uhp).ok());
                                let analysis_tree = analysis_tree_from_sources(
                                    uhp_state.clone(),
                                    game_response.as_ref(),
                                    move_number,
                                );
                                let vertical = vertical();
                                let game_type = analysis_tree.game_type;
                                let analysis_signal = AnalysisStore(Store::new(analysis_tree));
                                game_state.view().set(crate::providers::game_state::View::Game);
                                let mut state = uhp_state
                                    .or_else(|| {
                                        game_response
                                            .as_ref()
                                            .map(|game_response| game_response.create_state())
                                    })
                                    .unwrap_or_else(|| State::new(game_type, false));
                                state.game_status = GameStatus::InProgress;
                                if let Some(m) = move_number {
                                    let n_history_moves = state.history.moves.len();
                                    if m < n_history_moves {
                                        state.undo(n_history_moves - m - 1);
                                    }
                                }
                                let history_turn = if state.history.moves.is_empty() {
                                    None
                                } else {
                                    Some(state.history.moves.len())
                                };
                                game_state.history_turn().set(history_turn);
                                game_state.state().set(state);
                                provide_context(analysis_signal);
                                Either::Right(view! { <AnalysisContent vertical /> })
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn AnalysisContent(vertical: bool) -> impl IntoView {
    if vertical {
        Either::Left(view! {
            <div class="flex flex-col h-[85dvh]">
                <div class="flex flex-col flex-grow shrink">
                    <div class="flex justify-between h-full max-h-16">
                        <Reserve alignment=Alignment::SingleRow color=Color::White />
                    </div>
                </div>
                <AnalysisInfo />
                <Board />
                <div class="flex flex-col flex-grow shrink">
                    <div class="flex justify-between h-full max-h-16">
                        <Reserve alignment=Alignment::SingleRow color=Color::Black />
                    </div>
                </div>
            </div>
            <History mobile=true />
        })
    } else {
        Either::Right(view! {
            <AnalysisInfo extend_tw_classes="absolute pl-4 pt-2 bg-transparent" />
            <Board />
            <div class="flex flex-col col-span-2 row-span-6 p-1 h-full border-2 border-black select-none dark:border-white">
                <History />
            </div>
        })
    }
}

#[component]
fn AnalysisInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let analysis = expect_context::<AnalysisStore>();
    let game_state = expect_context::<GameStateStore>();
    let moves = move || {
        let mut moves = Vec::new();
        let current_node = analysis.current_node().get();
        let tree = analysis.tree().get();
        let sibling_nodes = current_node
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
            })) = s.get_value()
            {
                let analysis = analysis.clone();
                moves.push(
                    view! {
                        <div
                            class="underline cursor-pointer active:scale-95 no-link-style hover:text-pillbug-teal"
                            on:click=move |_| {
                                if let Ok(node_id) = s.get_node_id() {
                                    analysis.update_node(node_id);
                                    analysis.sync_game_state(game_state);
                                }
                            }
                        >
                            {format!("{turn}. {piece} {position}")}
                        </div>
                    },
                );
            }
        }
        moves
    };
    view! {
        <div class=extend_tw_classes>
            <div class="flex gap-1 items-center">
                <b>"Other Lines Explored: "</b>
                {move || moves}
            </div>
        </div>
    }
}
