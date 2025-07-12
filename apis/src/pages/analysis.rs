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
        analysis::{AnalysisSignal, AnalysisTree, TreeNode},
        game_state::GameStateSignal,
        AuthContext,
    },
    responses::GameResponse,
};
use hive_lib::{Color, GameStatus, GameType};
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
                "pt-12 bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
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
                    let analysis_signal = game_resource
                        .with(|gr| {
                            match gr {
                                Some(
                                    Ok(game_response),
                                ) if !should_block_analysis(game_response) => {
                                    let analysis_tree = AnalysisTree::from_game_response(
                                        game_response,
                                        game_state,
                                        move_number.get_value(),
                                    );
                                    AnalysisSignal(
                                        RwSignal::new(
                                            LocalStorage::wrap(analysis_tree.unwrap_or_default()),
                                        ),
                                    )
                                }
                                _ => {
                                    let analysis_tree = AnalysisTree::new_blank_analysis(
                                        game_state,
                                        GameType::MLP,
                                    );
                                    AnalysisSignal(RwSignal::new(LocalStorage::wrap(analysis_tree)))
                                }
                            }
                        });
                    provide_context(analysis_signal);

                    view! {
                        <Show
                            when=vertical
                            fallback=move || {
                                view! {
                                    <AnalysisInfo extend_tw_classes="absolute pl-4 pt-2 bg-board-dawn dark:bg-board-twilight" />
                                    <Board />
                                    <div class="flex flex-col col-span-2 row-span-6 p-1 h-full border-2 border-black select-none dark:border-white">
                                        <History />
                                    </div>
                                }
                            }
                        >
                            <div class="flex flex-col h-[85dvh]">
                                <div class="flex flex-col flex-grow shrink">
                                    <div class="flex justify-between h-full max-h-16">
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::White
                                        />
                                    </div>
                                </div>
                                <AnalysisInfo />
                                <Board />
                                <div class="flex flex-col flex-grow shrink">
                                    <div class="flex justify-between h-full max-h-16">
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::Black
                                        />
                                    </div>
                                </div>
                            </div>
                            <History mobile=true />
                        </Show>
                    }
                }}
            </Suspense>
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
                .and_then(|n| tree.get_sibling_ids(&n.get_node_id(), false).ok())
                .map_or(Vec::new(), |s| {
                    s.iter()
                        .filter_map(|id| tree.get_node_by_id(id))
                        .collect::<Vec<_>>()
                });
            for s in sibling_nodes {
                let TreeNode {
                    turn,
                    piece,
                    position,
                } = s.get_value().unwrap();
                moves.push(
                        view! {
                            <div
                                class="underline cursor-pointer no-link-style hover:text-pillbug-teal active:scale-95"
                                on:click=move |_| {
                                    analysis
                                        .update(|a| {
                                            a.update_node(s.get_node_id(), Some(game_state));
                                        })
                                }
                            >
                                {format!("{turn}. {piece} {position}")}
                            </div>
                        }
                    );
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
