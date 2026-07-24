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
    hiveground::{analysis_hiveground_interaction, selected_history_board},
    hooks::history_nav::use_analysis_history_keyboard_navigation,
    providers::{
        analysis::{AnalysisContext, AnalysisStore},
        annotations::AnnotationsSignal,
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        AuthIdentity,
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
use std::time::Duration;

const MOBILE_RESERVE_SYNC_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Eq, PartialEq)]
enum AnalysisSource {
    Blank,
    Uhp(String),
    Game(GameId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AnalysisLoadState {
    Loading(AnalysisSource),
    Ready(AnalysisSource),
    Failed {
        source: AnalysisSource,
        message: String,
    },
}

impl AnalysisLoadState {
    fn source(&self) -> &AnalysisSource {
        match self {
            Self::Loading(source) | Self::Ready(source) | Self::Failed { source, .. } => source,
        }
    }

    fn is_ready(&self, source: &AnalysisSource) -> bool {
        matches!(self, Self::Ready(loaded) if loaded == source)
    }

    fn is_loading(&self, source: &AnalysisSource) -> bool {
        self.source() != source || matches!(self, Self::Loading(loading) if loading == source)
    }

    fn error(&self, source: &AnalysisSource) -> Option<String> {
        match self {
            Self::Failed {
                source: failed,
                message,
            } if failed == source => Some(message.clone()),
            _ => None,
        }
    }

    fn can_retry(&self, source: &AnalysisSource) -> bool {
        matches!(self, Self::Failed { source: failed, .. } if failed == source)
            && matches!(source, AnalysisSource::Game(_))
    }
}

#[component]
pub fn Analysis() -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let history_board = selected_history_board(game_state);
    let params = use_params_map();
    let queries = use_query_map();
    let game_id = Memo::new(move |_| params().get("nanoid").map(|s| GameId(s.to_owned())));
    let requested_ply = Memo::new(move |_| {
        queries
            .get()
            .get("move")
            .and_then(|s| s.parse::<usize>().ok())
    });
    let uhp_string = Memo::new(move |_| queries.get().get("uhp"));
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let preview_snapshot = RwSignal::new(None::<AnalysisPreviewSnapshot>);
    let state = game_state.state();
    let turn_color = Memo::new(move |_| state.with(|state| state.turn_color));
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
    let analysis_store = AnalysisStore::new_blank(game_state, GameType::MLP);
    let analysis = AnalysisContext::new(
        analysis_store,
        sync_reserve,
        hold_reserve_sync,
        sync_reserve_later,
    );
    provide_context(analysis);
    provide_context(AnnotationsSignal::analysis(analysis));
    let hiveground_interaction = analysis_hiveground_interaction();

    use_analysis_history_keyboard_navigation(
        move || Some(analysis),
        move |analysis| {
            analysis.sync_reserve_later_from_game_state(game_state);
            reset_analysis_preview(preview_snapshot, analysis, game_state);
        },
    );

    let game_response = game_state.game_response();
    let has_game_response = Memo::new(move |_| game_response.with(Option::is_some));
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
    let mobile_player_panel = move |color: Color| {
        view! {
            <div class=mobile_panel_class(color)>
                <div class=mobile_reserve_row_class(color)>
                    <Reserve
                        alignment=Alignment::SingleRow
                        color
                        viewbox_str=MOBILE_RESERVE_VIEWBOX
                        interaction=hiveground_interaction
                        history_board
                    />
                </div>
                <div class=mobile_player_info_class(color)>
                    <div class="min-w-0">
                        <UserWithRating side=color vertical=true />
                    </div>
                    <div class=mobile_game_info_class(color)>
                        <GameInfo compact=true />
                    </div>
                </div>
            </div>
        }
    };

    let game_resource = Resource::new(game_id, move |game_id| async move {
        match game_id {
            Some(game_id) => {
                let requested_game_id = game_id.clone();
                Some((requested_game_id, get_game_from_nanoid(game_id).await))
            }
            None => None,
        }
    });
    let requested_source = Memo::new(move |_| {
        if let Some(game_id) = game_id.get() {
            AnalysisSource::Game(game_id)
        } else if let Some(uhp) = uhp_string.get() {
            AnalysisSource::Uhp(uhp)
        } else {
            AnalysisSource::Blank
        }
    });
    let load_state = RwSignal::new(AnalysisLoadState::Ready(AnalysisSource::Blank));
    let complete_load = Callback::new(
        move |(source, result): (AnalysisSource, Result<(), String>)| match result {
            Ok(()) => {
                preview_snapshot.set(None);
                analysis.sync_reserve_from_game_state(game_state);
                load_state.set(AnalysisLoadState::Ready(source));
            }
            Err(message) => load_state.set(AnalysisLoadState::Failed { source, message }),
        },
    );
    let auth_pending = Memo::new(move |_| auth_context.identity.get().is_none());
    let blocked = Memo::new(move |_| {
        let Some(identity) = auth_context.identity.get() else {
            return false;
        };
        requested_source.with(|source| {
            let AnalysisSource::Game(requested_game_id) = source else {
                return false;
            };
            game_response.with(|game_response| {
                game_response.as_ref().is_some_and(|game_response| {
                    game_response.game_id == *requested_game_id
                        && should_block_analysis(identity, game_response)
                })
            })
        })
    });

    Effect::watch(
        requested_ply,
        move |ply, _, _| {
            let is_ready = requested_source
                .with_untracked(|source| load_state.with_untracked(|state| state.is_ready(source)));
            if is_ready {
                reset_analysis_preview(preview_snapshot, analysis, game_state);
                if analysis_store.select_main_ply(*ply, game_state) {
                    analysis.sync_reserve_from_game_state(game_state);
                }
            }
        },
        false,
    );
    let analysis_available = move || {
        !auth_pending.get()
            && !blocked.get()
            && requested_source.with(|source| load_state.with(|state| state.is_ready(source)))
    };
    let loading = move || {
        auth_pending.get()
            || requested_source.with(|source| load_state.with(|state| state.is_loading(source)))
    };
    let load_error =
        move || requested_source.with(|source| load_state.with(|state| state.error(source)));
    let can_retry = move || {
        !auth_pending.get()
            && !blocked.get()
            && requested_source.with(|source| load_state.with(|state| state.can_retry(source)))
    };
    let retry = move |_| {
        let source = requested_source.get_untracked();
        load_state.set(AnalysisLoadState::Loading(source));
        game_resource.refetch();
    };
    let install_source = move || {
        let source = requested_source.get();
        Suspend::new(async move {
            if load_state.with_untracked(|state| state.is_ready(&source)) {
                return;
            }
            if !load_state.with_untracked(
                |state| matches!(state, AnalysisLoadState::Loading(loading) if loading == &source),
            ) {
                load_state.set(AnalysisLoadState::Loading(source.clone()));
            }
            let result = match &source {
                AnalysisSource::Blank => {
                    analysis_store.reset_with_game_type(game_state, GameType::MLP);
                    Ok(())
                }
                AnalysisSource::Uhp(uhp) => analysis_store
                    .load_uhp(game_state, uhp, requested_ply.get_untracked())
                    .map_err(|error| format!("Could not load UHP analysis: {error}")),
                AnalysisSource::Game(requested_game_id) => {
                    let resource = game_resource.by_ref().await;
                    if requested_source.with_untracked(|requested| requested != &source) {
                        return;
                    }
                    let Some((resource_game_id, result)) = resource.as_ref() else {
                        return;
                    };
                    if resource_game_id != requested_game_id {
                        return;
                    }
                    match result {
                        Err(error) => Err(format!("Could not load analysis: {error}")),
                        Ok(game_response) if game_response.game_id != *requested_game_id => {
                            Err("Could not load the requested analysis.".to_string())
                        }
                        Ok(game_response) => analysis_store
                            .load_game_response(
                                game_state,
                                game_response,
                                requested_ply.get_untracked(),
                            )
                            .map_err(|error| format!("Could not load analysis: {error}")),
                    }
                }
            };
            if requested_source.with_untracked(|requested| requested != &source) {
                return;
            }
            complete_load.run((source, result));
        })
    };

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
                view! {
                    <div class="flex col-span-full row-span-full justify-center items-center p-4 min-h-[calc(100dvh-2.5rem)]">
                        <div class="max-w-md ui-empty-state" role="status">
                            <div class="text-sm font-bold text-gray-800 dark:text-gray-100">
                                "Loading analysis..."
                            </div>
                        </div>
                    </div>
                }
            }>
                {install_source}
                <Show
                    when=analysis_available
                    fallback=move || {
                        let title = if blocked.get() {
                            "Analysis is unavailable during this game."
                        } else if loading() {
                            "Loading analysis..."
                        } else {
                            "Could not load analysis."
                        };
                        view! {
                            <div class="flex col-span-full row-span-full justify-center items-center p-4 min-h-[calc(100dvh-2.5rem)]">
                                <div class="max-w-md ui-empty-state" role="status">
                                    <div class="text-sm font-bold text-gray-800 dark:text-gray-100">
                                        {title}
                                    </div>
                                    <ShowLet some=load_error let:message>
                                        <div class="mt-1 text-xs">{message}</div>
                                    </ShowLet>
                                    <Show when=can_retry>
                                        <button
                                            type="button"
                                            class="mt-3 ui-button ui-button-primary ui-button-sm"
                                            on:click=retry
                                        >
                                            "Retry"
                                        </button>
                                    </Show>
                                </div>
                            </div>
                        }
                    }
                >
                    <For
                        each=move || std::iter::once(analysis_store.document_generation())
                        key=|generation| *generation
                        children=move |_| {
                            view! {
                                <Show
                                    when=vertical
                                    fallback=move || {
                                        view! {
                                            <div class="grid relative grid-cols-8 col-span-8 col-start-1 grid-rows-6 row-span-6 row-start-1 min-w-0 min-h-0">
                                                <Board interaction=hiveground_interaction history_board />
                                                <VariationList extend_tw_classes="absolute left-1 top-1 z-20" />
                                            </div>
                                            <div class="flex flex-col col-span-2 row-span-6 gap-2 my-1 mr-1 min-h-0">
                                                <GameDetailsPanel />
                                                <AnalysisSidebar
                                                    interaction=hiveground_interaction
                                                    history_board
                                                    preview_snapshot
                                                />
                                            </div>
                                        }
                                    }
                                >
                                    <div class="flex flex-col min-h-0 h-[calc(100svh-2.5rem)]">
                                        {mobile_player_panel(Color::White)}
                                        <div class="flex relative min-h-0 grow">
                                            <Board interaction=hiveground_interaction history_board />
                                            <VariationList extend_tw_classes="absolute left-1 top-1 z-20" />
                                        </div> {mobile_player_panel(Color::Black)}
                                        <div class="order-2 shrink-0 ui-board-reserve">
                                            <AnalysisMobileHistoryControls />
                                        </div>
                                    </div>
                                    <AnalysisMobileTabs
                                        interaction=hiveground_interaction
                                        history_board
                                        preview_snapshot
                                    />
                                </Show>
                            }
                        }
                    />
                </Show>
            </Suspense>
        </div>
    }
}

fn should_block_analysis(identity: AuthIdentity, game_response: &GameResponse) -> bool {
    let AuthIdentity::User(user_id) = identity else {
        return false;
    };
    game_response.rated
        && matches!(game_response.game_status, GameStatus::InProgress)
        && game_response.time_mode == TimeMode::RealTime
        && (user_id == game_response.white_player.uid || user_id == game_response.black_player.uid)
}
