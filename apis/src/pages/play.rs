use crate::{
    common::{CurrentConfirm, GameReaction, PieceType},
    components::{
        layouts::base_layout::{ControlsSignal, OrientationSignal},
        molecules::{
            analysis_and_download::AnalysisAndDownload,
            annotation_toolbar::AnnotationToggle,
            control_buttons::ControlButtons,
            game_info::GameInfo,
            play_history_button::{HistoryButton, PlayHistoryNavigation as HistoryNavigation},
            user_with_rating::UserWithRating,
        },
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve, MOBILE_RESERVE_VIEWBOX},
            side_board::{move_query_signal, SideboardTabs, TabView},
            unstarted::Unstarted,
        },
    },
    functions::games::get::get_game_from_nanoid,
    hiveground::{live_hiveground_interaction, selected_history_state, HivegroundInteraction},
    hooks::history_nav::{
        scroll_move_into_view,
        sync_play_move_query,
        use_play_history_keyboard_navigation,
    },
    providers::{
        annotations::AnnotationsSignal,
        config::Config,
        game_state::{GameStateSignal, View},
        timer::TimerSignal,
        websocket::WebsocketContext,
        ApiRequestsProvider,
        AuthContext,
        SoundType,
        Sounds,
        UpdateNotifier,
    },
    websocket::client_handlers::game::{reset_game_state, reset_game_state_for_takeback},
};
use hive_lib::{Color, GameControl, GameResult, GameStatus, State as HiveState, Turn};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{GameId, GameStart};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn Play() -> impl IntoView {
    provide_context(TimerSignal::new());
    let timer = expect_context::<TimerSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let vertical = orientation_signal.orientation_vertical;
    let height_lock = orientation_signal.height_lock;
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let api = expect_context::<ApiRequestsProvider>();
    let game_updater = expect_context::<UpdateNotifier>();
    let sounds = expect_context::<Sounds>();
    let ws = expect_context::<WebsocketContext>();
    let controls_signal = expect_context::<ControlsSignal>();
    let ws_ready = ws.ready_state;
    let params = use_params_map();
    let queries = use_query_map();
    let (_move, set_move) = move_query_signal();
    let move_number = Signal::derive(move || {
        queries
            .get()
            .get("move")
            .and_then(|s| s.parse::<usize>().ok())
            .map(|n| n.saturating_sub(1))
    });
    let game_id = Memo::new(move |_| {
        params()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    });
    let tab = RwSignal::new(TabView::Reserve);
    let current_confirm = Memo::new(move |_| {
        config.with(|cfg| {
            let preferred_confirms = &cfg.confirm_mode;
            game_state
                .signal
                .with(|gs| gs.get_game_speed())
                .and_then(|game_speed| preferred_confirms.get(&game_speed).cloned())
                .unwrap_or_default()
        })
    });
    provide_context(CurrentConfirm(current_confirm));
    provide_context(AnnotationsSignal::play(game_state));
    let hiveground_interaction = live_hiveground_interaction();
    let history_state = selected_history_state(game_state);
    let user = auth_context.user;
    let white_and_black_ids = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));
    let user_is_player = Signal::derive(move || {
        user.with(|a| {
            if let Some(user) = a {
                let (white_id, black_id) = white_and_black_ids();
                Some(user.id) == white_id || Some(user.id) == black_id
            } else {
                false
            }
        })
    });
    let player_color = Memo::new(move |_| {
        user.with(|a| {
            a.as_ref().map_or(Color::White, |user| {
                let black_id = white_and_black_ids().1;
                match Some(user.id) == black_id {
                    true => Color::Black,
                    false => Color::White,
                }
            })
        })
    });
    let parent_container_style = move || {
        if vertical.get() {
            "flex flex-col"
        } else {
            "grid grid-cols-board xl:grid-cols-board-xl grid-rows-6 pr-2"
        }
    };
    let page_class = move || {
        let height_class = if vertical.get() && height_lock.get().is_some() {
            "h-[var(--locked-app-height)]"
        } else {
            "h-[100dvh] standalone:h-[var(--app-height)]"
        };
        format!("select-none ui-board-page-surface {height_class}")
    };
    let page_height_style = move || {
        if vertical.get() {
            height_lock
                .get()
                .map(|height| format!("--locked-app-height: {height}px;"))
                .unwrap_or_default()
        } else {
            String::new()
        }
    };

    let show_board = create_read_slice(game_state.signal, |gs| {
        !gs.game_response.as_ref().is_some_and(|gr| {
            gr.game_start == GameStart::Ready && matches!(gr.game_status, GameStatus::NotStarted)
        })
    });

    //HB handler
    Effect::watch(
        move || game_updater.heartbeat.get(),
        move |hb, _, _| {
            timer.update_from_hb(hb.clone());
        },
        false,
    );

    let timer_display_key = create_read_slice(game_state.signal, |gs| {
        let history_turn = gs.history_turn;
        let gs_view = gs.view.clone();
        let response = gs.game_response.as_ref().map(|response| {
            (
                response.game_id.clone(),
                response.updated_at,
                response.conclusion.clone(),
                response.game_status.clone(),
                response.time_mode,
            )
        });
        (gs_view, history_turn, response)
    });

    Effect::watch(
        timer_display_key,
        move |_, _, _| {
            game_state.signal.with_untracked(|gs| {
                if !matches!(
                    gs.state.game_status,
                    GameStatus::Finished(_) | GameStatus::Adjudicated
                ) {
                    return;
                }
                if let Some(response) = gs.game_response.as_ref() {
                    timer.signal.update(|timer| {
                        timer.update_for_view(response, &gs.view, gs.history_turn);
                    });
                }
            });
        },
        true,
    );

    // Unsubscribe this socket from the game when the component unmounts
    // (route change away from the game page). Without this, the socket stays
    // in games_sockets for the lifetime of the WebSocket session.
    on_cleanup(move || {
        let current_game_id = game_id.get_untracked();
        if !current_game_id.0.is_empty() {
            api.0.get().unwatch(current_game_id);
        }
    });

    Effect::watch(
        move || {
            ws_ready();
            game_id()
        },
        move |game_id, prev_game_id, _| {
            // Route param can change in place (e.g. /game/A → /game/B reuses
            // the same component), so on_cleanup won't fire. Drop the prior
            // subscription before joining the new game.
            if let Some(prev) = prev_game_id {
                if !prev.0.is_empty() && prev != game_id {
                    api.0.get().unwatch(prev.clone());
                }
            }
            let game_id = game_id.clone();
            api.0.get().join(game_id.clone());
            spawn_local(async move {
                let game = get_game_from_nanoid(game_id).await;
                if let Ok(game) = game {
                    reset_game_state(&game, game_state);
                    timer.update_from(&game);
                    if let Some((_turn, gc)) = game.game_control_history.last() {
                        match gc {
                            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                                game_state.set_pending_gc(*gc)
                            }
                            _ => {}
                        }
                    }
                    let url_number = move_number.get_untracked();
                    if url_number.is_some_and(|v| {
                        game_state
                            .signal
                            .with_untracked(|gs| v < gs.state.turn.saturating_sub(1))
                    }) {
                        game_state.signal.update(|s| {
                            s.history_turn = url_number;
                            s.view = View::History;
                        });
                        controls_signal.hidden.set(false);
                        tab.set(TabView::History);
                    }
                };
            });
        },
        true,
    );

    Effect::watch(
        game_updater.game_response,
        move |gar, _, _| {
            if let Some(gar) = gar {
                let game_id = game_id.get_untracked();
                if gar.game_id == game_id {
                    match gar.game_action.clone() {
                        GameReaction::Turn(turn) => {
                            timer.update_from(&gar.game);
                            game_state.clear_gc();
                            game_state.set_game_response(gar.game.clone());
                            sounds.play_sound(SoundType::Turn);
                            let (pos, reserve_pos, history_moves, active, was_at_live_edge) =
                                game_state.signal.with_untracked(|gs| {
                                    (
                                        gs.move_info.current_position,
                                        gs.move_info.reserve_position,
                                        gs.state.history.moves.clone(),
                                        gs.move_info.active,
                                        matches!(gs.view, View::History) && gs.is_last_turn(),
                                    )
                                });
                            if history_moves != gar.game.history {
                                match turn {
                                    Turn::Move(piece, position) => {
                                        game_state.play_turn(piece, position);
                                        if was_at_live_edge {
                                            game_state.view_game();
                                            sync_play_move_query(game_state, &set_move);
                                        }
                                        if let Some((piece, piece_type)) = active {
                                            match piece_type {
                                                PieceType::Board => {
                                                    if let Some(position) = pos {
                                                        game_state.show_moves(piece, position);
                                                    }
                                                }
                                                PieceType::Inactive => {
                                                    if let Some(position) = reserve_pos {
                                                        game_state.show_spawns(piece, position);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => unreachable!(),
                                };
                            }
                        }
                        GameReaction::Control(game_control) => {
                            game_state.set_pending_gc(game_control);

                            match game_control {
                                GameControl::DrawAccept(_) => {
                                    game_state
                                        .set_game_status(GameStatus::Finished(GameResult::Draw));
                                    game_state.set_game_response(gar.game.clone());
                                    timer.update_from(&gar.game);
                                }
                                GameControl::Resign(color) => {
                                    game_state.set_game_status(GameStatus::Finished(
                                        GameResult::Winner(color.opposite_color()),
                                    ));
                                    game_state.set_game_response(gar.game.clone());
                                    timer.update_from(&gar.game);
                                }
                                GameControl::TakebackAccept(_) => {
                                    timer.update_from(&gar.game);
                                    reset_game_state_for_takeback(&gar.game, &mut game_state);
                                }
                                GameControl::TakebackRequest(_)
                                | GameControl::DrawOffer(_)
                                | GameControl::DrawReject(_)
                                | GameControl::TakebackReject(_)
                                | GameControl::Abort(_) => {}
                            };
                        }
                        GameReaction::Started => {
                            reset_game_state(&gar.game, game_state);
                            timer.update_from(&gar.game);
                        }
                        GameReaction::TimedOut => {
                            reset_game_state(&gar.game, game_state);
                            timer.update_from(&gar.game);
                        }
                        GameReaction::Join => {
                            // TODO: Do we want anything here?
                        }
                        _ => {
                            todo!()
                        }
                    }
                }
            }
        },
        false,
    );

    let on_history_key = Callback::new(move |action: HistoryNavigation| {
        if action == HistoryNavigation::Previous {
            if tab.get_untracked() != TabView::Chat {
                tab.set(TabView::History);
            }
            controls_signal.hidden.set(false);
        }
        scroll_move_into_view();
    });
    use_play_history_keyboard_navigation(game_state, set_move, on_history_key);

    view! {
        <div class=page_class style=page_height_style>
            <div class=move || format!("h-full {}", parent_container_style())>
                <Show
                    when=vertical
                    fallback=move || {
                        view! {
                            <HorizontalLayout
                                show_board
                                player_color
                                user_is_player
                                game_id
                                white_and_black_ids
                                interaction=hiveground_interaction
                                tab
                                history_state
                            />
                        }
                    }
                >
                    <VerticalLayout
                        show_board
                        player_color
                        user_is_player
                        game_id
                        white_and_black_ids
                        interaction=hiveground_interaction
                        history_state
                    />

                </Show>
            </div>
        </div>
    }
}

#[component]
fn BoardOrUnstarted(
    show_board: Signal<bool>,
    user_is_player: Signal<bool>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    game_id: Memo<GameId>,
    interaction: HivegroundInteraction,
    history_state: Memo<HiveState>,
) -> impl IntoView {
    let game_updater = expect_context::<UpdateNotifier>();
    view! {
        <Show
            when=show_board
            fallback=move || {
                view! {
                    <Unstarted
                        user_is_player
                        ready=game_updater.tournament_ready
                        game_id
                        white_and_black_ids
                    />
                }
            }
        >
            <Board interaction history_state />
        </Show>
    }
}

#[component]
fn HorizontalLayout(
    show_board: Signal<bool>,
    player_color: Memo<Color>,
    user_is_player: Signal<bool>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    game_id: Memo<GameId>,
    interaction: HivegroundInteraction,
    tab: RwSignal<TabView>,
    history_state: Memo<HiveState>,
) -> impl IntoView {
    let vertical = false;
    let config = expect_context::<Config>().0;
    let background_style = Signal::derive(move || {
        let bg = config.with(|c| c.tile.get_effective_background_color(c.prefers_dark));
        format!("background-color: {bg}")
    });
    view! {
        <GameInfo extend_tw_classes="relative z-10 col-start-1 row-start-1 col-span-8 self-start overflow-hidden min-w-0 pr-2 pl-4 pt-2 bg-transparent pointer-events-none" />
        <BoardOrUnstarted
            show_board
            user_is_player
            game_id
            white_and_black_ids
            interaction
            history_state
        />
        <div
            class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full row-start-1 gap-2 p-1"
            style=background_style
        >
            <DisplayTimer placement=Placement::Top vertical />
            <SideboardTabs player_color tab interaction history_state />
            <DisplayTimer placement=Placement::Bottom vertical />
        </div>
    }
}

#[component]
fn VerticalLayout(
    show_board: Signal<bool>,
    player_color: Memo<Color>,
    user_is_player: Signal<bool>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    game_id: Memo<GameId>,
    interaction: HivegroundInteraction,
    history_state: Memo<HiveState>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let controls_signal = expect_context::<ControlsSignal>();
    let vertical = true;
    let top_color = Signal::derive(move || player_color().opposite_color());
    let show_controls =
        Signal::derive(move || !controls_signal.hidden.get() || game_state.is_finished()());
    let top_reserve_row_class = move || {
        let size_class = if user_is_player() {
            "h-12 max-h-12"
        } else {
            "h-16 max-h-16"
        };
        format!("flex col-start-1 row-start-1 justify-between ml-1 min-w-0 {size_class}")
    };
    let bottom_reserve_row_class = move || {
        let size_class = if user_is_player() {
            "h-20 max-h-20"
        } else {
            "h-16 max-h-16"
        };
        format!("flex col-start-1 row-start-2 justify-between ml-1 min-w-0 {size_class}")
    };
    view! {
        <div class="flex flex-col h-full min-h-0 grow">
            <div class="flex flex-col shrink ui-board-reserve">
                <Show when=show_controls>
                    <div class="flex flex-row-reverse justify-between items-start min-w-0 shrink">
                        <div class="flex flex-row-reverse items-center">
                            <AnalysisAndDownload />
                        </div>
                        <Show when=user_is_player>
                            <ControlButtons />
                        </Show>
                    </div>
                </Show>

                <div class="grid grid-cols-[minmax(0,1fr)_4rem] bg-inherit">
                    <div class=top_reserve_row_class>
                        <Reserve
                            alignment=Alignment::SingleRow
                            color=top_color
                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                            interaction
                            history_state
                        />
                    </div>
                    <div class="flex col-start-2 row-start-1 min-h-0">
                        <DisplayTimer vertical=true placement=Placement::Top />
                    </div>
                </div>
                <div class="grid gap-y-0.5 gap-x-1 items-start px-1 min-w-0 grid-cols-[auto_minmax(0,1fr)] bg-inherit ui-board-separator-bottom">
                    <div class="min-w-0">
                        <UserWithRating side=top_color vertical />
                    </div>
                    <GameInfo compact=true />
                </div>

            </div>
            <BoardOrUnstarted
                show_board
                user_is_player
                game_id
                white_and_black_ids
                interaction
                history_state
            />
            <div class="flex flex-col shrink ui-board-reserve">
                <div class="grid grid-cols-[minmax(0,1fr)_4rem] bg-inherit ui-board-separator-top">
                    <div class="flex col-start-1 row-start-1 gap-1 min-w-0">
                        <UserWithRating side=player_color vertical />
                    </div>
                    <div class=bottom_reserve_row_class>
                        <Reserve
                            alignment=Alignment::SingleRow
                            color=player_color
                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                            interaction
                            history_state
                        />
                    </div>
                    <div class="flex col-start-2 row-span-2 row-start-1 min-h-0">
                        <DisplayTimer vertical=true placement=Placement::Bottom />
                    </div>
                </div>
                <Show when=show_controls>
                    <div class="grid grid-cols-5 gap-1 px-1 pb-1 *:w-full">
                        <HistoryButton action=HistoryNavigation::First />
                        <HistoryButton action=HistoryNavigation::Previous />
                        <HistoryButton action=HistoryNavigation::Next />
                        <HistoryButton action=HistoryNavigation::Last />
                        <AnnotationToggle
                            class="ui-board-nav-button"
                            active_tw_classes="ui-segmented-active"
                        />
                    </div>
                </Show>

            </div>
        </div>
    }
}
