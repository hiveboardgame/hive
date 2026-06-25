use crate::{
    common::{CurrentConfirm, MoveConfirm},
    components::{
        layouts::base_layout::OrientationSignal,
        molecules::{game_info::GameInfo, user_with_rating::UserWithRating},
        organisms::{
            analysis::{
                reset_analysis_preview,
                AnalysisMobileHistoryControls,
                AnalysisMobileTabs,
                AnalysisPreviewSnapshot,
                AnalysisSidebar,
                GameDetailsPanel,
                VariationList,
            },
            board::Board,
            reserve::{Alignment, Reserve, MOBILE_RESERVE_VIEWBOX},
        },
    },
    functions::games::get::get_game_from_nanoid,
    hiveground::{analysis_hiveground_interaction, selected_history_state},
    hooks::history_nav::use_analysis_history_keyboard_navigation,
    providers::{
        analysis::{AnalysisSignal, AnalysisTree},
        annotations::AnnotationsSignal,
        game_state::GameStateSignal,
        AuthContext,
    },
    responses::GameResponse,
};
use hive_lib::{Color, GameStatus, GameType};
use leptos::{
    leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
    prelude::*,
};
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{GameId, TimeMode};
use std::{collections::HashSet, time::Duration};

#[derive(Clone)]
pub struct ToggleStates(pub RwSignal<HashSet<i32>>);

const MOBILE_RESERVE_SYNC_DELAY: Duration = Duration::from_millis(500);

#[component]
pub fn Analysis() -> impl IntoView {
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
    let active_analysis = StoredValue::new(None::<AnalysisSignal>);
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let preview_snapshot = RwSignal::new(None::<AnalysisPreviewSnapshot>);
    let turn_color = create_read_slice(game_state.signal, |gs| gs.state.turn_color);
    let mobile_bottom_color = RwSignal::new(turn_color.get_untracked());
    let reserve_orientation_pending = RwSignal::new(false);
    let reserve_sync_timer = StoredValue::new(None::<TimeoutHandle>);
    let cancel_reserve_sync = Callback::new(move |_| {
        reserve_sync_timer.update_value(|timer| {
            if let Some(timer) = timer.take() {
                timer.clear();
            }
        });
    });
    on_cleanup(move || cancel_reserve_sync.run(()));
    let hold_reserve_sync = Callback::new(move |_| {
        if vertical.get_untracked() {
            reserve_orientation_pending.set(true);
            cancel_reserve_sync.run(());
        }
    });
    let sync_reserve = Callback::new(move |color| {
        cancel_reserve_sync.run(());
        reserve_orientation_pending.set(false);
        mobile_bottom_color.set(color);
    });
    let sync_reserve_later = Callback::new(move |color| {
        if vertical.get_untracked() {
            reserve_orientation_pending.set(true);
            cancel_reserve_sync.run(());
            if let Ok(handle) = set_timeout_with_handle(
                move || {
                    reserve_sync_timer.set_value(None);
                    sync_reserve.run(color);
                },
                MOBILE_RESERVE_SYNC_DELAY,
            ) {
                reserve_sync_timer.set_value(Some(handle));
            }
        } else {
            sync_reserve.run(color);
        }
    });

    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    Effect::watch(
        turn_color,
        move |_, _, _| {
            if !reserve_orientation_pending.get_untracked() {
                sync_reserve.run(turn_color.get_untracked());
            }
        },
        false,
    );
    use_analysis_history_keyboard_navigation(
        move || active_analysis.get_value(),
        move || !vertical.get_untracked(),
        move |analysis| {
            analysis.sync_reserve_later_from_game_state(game_state);
            reset_analysis_preview(preview_snapshot, analysis, game_state);
        },
    );

    let current_user_id =
        Signal::derive(move || auth_context.user.with(|u| u.as_ref().map(|user| user.id)));
    let has_game_response = create_read_slice(game_state.signal, |gs| gs.game_response.is_some());
    let mobile_panel_class = move |color: Color| {
        move || {
            format!(
                "flex flex-col shrink-0 ui-board-reserve {}",
                if mobile_bottom_color() == color {
                    "order-1"
                } else {
                    "-order-1"
                },
            )
        }
    };
    let mobile_player_info_class = move |color: Color| {
        move || {
            format!(
                "grid grid-cols-[auto_minmax(0,1fr)] gap-x-1 gap-y-0.5 items-start px-1 min-w-0 bg-inherit {} {}",
                if mobile_bottom_color() == color {
                    "order-1 ui-board-separator-top"
                } else {
                    "order-2 ui-board-separator-bottom"
                },
                if has_game_response() { "" } else { "hidden" },
            )
        }
    };
    let mobile_reserve_row_class = move |color: Color| {
        move || {
            let is_bottom = mobile_bottom_color() == color;
            let order_class = if is_bottom { "order-2" } else { "order-1" };
            let size_class = if is_bottom {
                "h-20 max-h-20"
            } else {
                "h-12 max-h-12"
            };
            format!("flex justify-between ml-1 min-w-0 bg-inherit {order_class} {size_class}",)
        }
    };
    let mobile_game_info_class = move |color: Color| {
        move || {
            if mobile_bottom_color() == color {
                "hidden min-w-0"
            } else {
                "min-w-0"
            }
        }
    };

    let should_block_analysis = move |game_response: &GameResponse| -> bool {
        let Some(user_id) = current_user_id() else {
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
                "ui-board-page-surface {}",
                if vertical() {
                    "flex h-full flex-col standalone:min-h-[var(--app-height)]"
                } else {
                    "grid min-h-[100dvh] max-h-[100dvh] grid-cols-10 grid-rows-6 pr-1 standalone:min-h-[var(--app-height)] standalone:max-h-[var(--app-height)]"
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
                            AnalysisSignal::new(
                                analysis_tree,
                                sync_reserve,
                                hold_reserve_sync,
                                sync_reserve_later,
                            )
                        });
                    provide_context(analysis_signal);
                    provide_context(AnnotationsSignal::analysis(analysis_signal));
                    active_analysis.set_value(Some(analysis_signal));
                    let hiveground_interaction = analysis_hiveground_interaction();

                    view! {
                        <Show
                            when=vertical
                            fallback=move || {
                                view! {
                                    <div class="grid relative grid-cols-8 col-span-8 col-start-1 grid-rows-6 row-span-6 row-start-1 min-w-0 min-h-0">
                                        <Board interaction=hiveground_interaction history_state />
                                        <VariationList extend_tw_classes="absolute left-1 top-1 z-20" />
                                    </div>
                                    <div class="flex flex-col col-span-2 row-span-6 gap-2 my-1 mr-1 min-h-0">
                                        <GameDetailsPanel />
                                        <AnalysisSidebar
                                            interaction=hiveground_interaction
                                            history_state
                                            preview_snapshot
                                        />
                                    </div>
                                }
                            }
                        >
                            <div class="flex flex-col min-h-0 h-[calc(100svh-2.5rem)]">
                                <div class=mobile_panel_class(Color::White)>
                                    <div class=mobile_reserve_row_class(Color::White)>
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::White
                                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                                            interaction=hiveground_interaction
                                            history_state
                                        />
                                    </div>
                                    <div class=mobile_player_info_class(Color::White)>
                                        <div class="min-w-0">
                                            <UserWithRating side=Color::White vertical=true />
                                        </div>
                                        <div class=mobile_game_info_class(Color::White)>
                                            <GameInfo compact=true />
                                        </div>
                                    </div>
                                </div>
                                <div class="flex relative min-h-0 grow">
                                    <Board interaction=hiveground_interaction history_state />
                                    <VariationList extend_tw_classes="absolute left-1 top-1 z-20" />
                                </div>
                                <div class=mobile_panel_class(Color::Black)>
                                    <div class=mobile_reserve_row_class(Color::Black)>
                                        <Reserve
                                            alignment=Alignment::SingleRow
                                            color=Color::Black
                                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                                            interaction=hiveground_interaction
                                            history_state
                                        />
                                    </div>
                                    <div class=mobile_player_info_class(Color::Black)>
                                        <div class="min-w-0">
                                            <UserWithRating side=Color::Black vertical=true />
                                        </div>
                                        <div class=mobile_game_info_class(Color::Black)>
                                            <GameInfo compact=true />
                                        </div>
                                    </div>
                                </div>
                                <div class="order-2 shrink-0 ui-board-reserve">
                                    <AnalysisMobileHistoryControls />
                                </div>
                            </div>
                            <AnalysisMobileTabs
                                interaction=hiveground_interaction
                                history_state
                                preview_snapshot
                            />
                        </Show>
                    }
                }}
            </Suspense>
        </div>
    }
}
