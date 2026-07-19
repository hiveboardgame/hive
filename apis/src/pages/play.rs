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
        game_state::{GameStateStore, GameStateStoreFields},
        timer::TimerSignal,
        websocket::{ConnectionReadyState, WebsocketContext},
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        SoundType,
        Sounds,
        UpdateNotifier,
    },
};
use hive_lib::{Color, GameControl, GameStatus, State as HiveState, Turn};
use leptos::{prelude::*, reactive::effect::batch, task::spawn_local_scoped_with_cancellation};
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{GameId, GameStart};
use uuid::Uuid;

#[component]
pub fn Play() -> impl IntoView {
    provide_context(TimerSignal::new());
    let timer = expect_context::<TimerSignal>();
    let game_state = expect_context::<GameStateStore>();
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
    let play_owner = Owner::current().expect("Play must run inside a reactive owner");
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
    let game_response = game_state.game_response();
    let current_confirm = Memo::new(move |_| {
        config.with(|cfg| {
            let preferred_confirms = &cfg.confirm_mode;
            game_response
                .with(|game_response| game_response.as_ref().map(|game| game.speed))
                .and_then(|game_speed| preferred_confirms.get(&game_speed).cloned())
                .unwrap_or_default()
        })
    });
    provide_context(CurrentConfirm(current_confirm));
    provide_context(AnnotationsSignal::play(game_state));
    let hiveground_interaction = live_hiveground_interaction();
    let history_state = selected_history_state(game_state);
    let identity = auth_context.identity;
    let white_id = game_state.white_id();
    let black_id = game_state.black_id();
    let white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)> =
        Memo::new(move |_| (white_id.get(), black_id.get())).into();
    let user_is_player = Signal::derive(move || {
        let user_id = identity.get().and_then(AuthIdentity::user_id);
        let (white_id, black_id) = white_and_black_ids();
        user_id.is_some() && (user_id == white_id || user_id == black_id)
    });
    let player_color = Memo::new(move |_| {
        let user_id = identity.get().and_then(AuthIdentity::user_id);
        if user_id.is_some() && user_id == white_and_black_ids().1 {
            Color::Black
        } else {
            Color::White
        }
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

    let game_response = game_state.game_response();
    let show_board: Signal<bool> = Memo::new(move |_| {
        !game_response.with(|game_response| {
            game_response.as_ref().is_some_and(|game| {
                game.game_start == GameStart::Ready
                    && matches!(game.game_status, GameStatus::NotStarted)
            })
        })
    })
    .into();

    //HB handler
    Effect::watch(
        move || game_updater.heartbeat.get(),
        move |hb, _, _| {
            timer.update_from_hb(hb.clone());
        },
        false,
    );

    let board_view = game_state.board_view();
    let game_response = game_state.game_response();
    let timer_display_key = Memo::new(move |_| {
        let board_view = board_view.get();
        let response = game_response.with(|game_response| {
            game_response.as_ref().map(|response| {
                (
                    response.game_id.clone(),
                    response.updated_at,
                    response.conclusion.clone(),
                    response.game_status.clone(),
                    response.time_mode,
                )
            })
        });
        (board_view, response)
    });

    Effect::watch(
        timer_display_key,
        move |_, _, _| {
            let is_finished = game_state.state().with_untracked(|state| {
                matches!(
                    state.game_status,
                    GameStatus::Finished(_) | GameStatus::Adjudicated
                )
            });
            if !is_finished {
                return;
            }
            let view = game_state.board_view().get_untracked();
            game_state.game_response().with_untracked(|response| {
                let Some(response) = response.as_ref() else {
                    return;
                };
                timer.signal.update(|timer| {
                    timer.update_for_view(response, &view);
                });
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
        move || (ws_ready(), game_id()),
        move |(ready_state, next_game_id), previous, _| {
            let previous_game_id = previous.map(|(_, game_id)| game_id);
            let route_changed = previous_game_id.is_none_or(|previous| previous != next_game_id);

            // Route param can change in place (e.g. /game/A → /game/B reuses
            // the same component), so on_cleanup won't fire. Drop the prior
            // subscription before joining the new game.
            if let Some(prev) = previous_game_id {
                if !prev.0.is_empty() && prev != next_game_id {
                    api.0.get().unwatch(prev.clone());
                }
            }
            // The app-scoped store can retain state across page mounts, while
            // parameter-only navigation can reuse this component.
            if route_changed {
                batch(|| {
                    game_state.full_reset();
                    timer.signal.set(Default::default());
                    controls_signal.hidden.set(true);
                    tab.set(TabView::Reserve);
                });
            }

            let connection_opened = *ready_state == ConnectionReadyState::Open
                && previous.is_none_or(|(previous_state, _)| {
                    *previous_state != ConnectionReadyState::Open
                });
            if *ready_state != ConnectionReadyState::Open || (!route_changed && !connection_opened)
            {
                return;
            }

            let requested_game_id = next_game_id.clone();
            api.0.get().join(requested_game_id.clone());
            play_owner.with(|| {
                spawn_local_scoped_with_cancellation(async move {
                    let game = get_game_from_nanoid(requested_game_id.clone()).await;
                    if let Ok(game) = game {
                        if requested_game_id != game_id.get_untracked() {
                            return;
                        }
                        batch(|| {
                            game_state.reset_from_response(&game);
                            timer.update_from(&game);
                            let url_number = move_number.get_untracked();
                            let state_turn = game_state.state().with_untracked(|state| state.turn);
                            if let Some(url_number) =
                                url_number.filter(|turn| *turn < state_turn.saturating_sub(1))
                            {
                                game_state.show_history_turn(url_number);
                                controls_signal.hidden.set(false);
                                tab.set(TabView::History);
                            }
                        });
                    };
                });
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
                            sounds.play_sound(SoundType::Turn);
                            let (
                                pos,
                                reserve_pos,
                                history_moves,
                                active,
                                board_view,
                                was_at_history_edge,
                            ) = game_state.with_untracked(|state| {
                                (
                                    state.move_info.current_position,
                                    state.move_info.reserve_position,
                                    state.state.history.moves.clone(),
                                    state.move_info.active,
                                    state.board_view.clone(),
                                    state.board_view.is_history()
                                        && state.board_view.is_last_turn(state.state.turn),
                                )
                            });
                            batch(|| {
                                timer.update_from(&gar.game);
                                if gar.game.finished {
                                    game_state.reset_from_response(&gar.game);
                                    if board_view.is_history() {
                                        if was_at_history_edge && history_moves != gar.game.history
                                        {
                                            sync_play_move_query(game_state, &set_move);
                                        } else {
                                            game_state.board_view().set(board_view);
                                        }
                                    }
                                    return;
                                }

                                game_state.game_control_pending().set(None);
                                game_state.set_game_response(gar.game.clone());
                                if history_moves != gar.game.history {
                                    match turn {
                                        Turn::Move(piece, position) => {
                                            game_state.play_turn(piece, position);
                                        }
                                        Turn::Shutout => unreachable!(),
                                    }
                                    if was_at_history_edge {
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
                            });
                        }
                        GameReaction::Control(game_control) => {
                            let board_view = game_state.board_view().get_untracked();
                            batch(|| {
                                if gar.game.finished {
                                    game_state.reset_from_response(&gar.game);
                                    if board_view.is_history() {
                                        game_state.board_view().set(board_view);
                                    }
                                    timer.update_from(&gar.game);
                                } else {
                                    match game_control {
                                        GameControl::TakebackAccept(_) => {
                                            timer.update_from(&gar.game);
                                            game_state.reset_from_response(&gar.game);
                                        }
                                        GameControl::TakebackRequest(_)
                                        | GameControl::DrawOffer(_) => {
                                            game_state
                                                .game_control_pending()
                                                .set(Some(game_control));
                                        }
                                        GameControl::DrawAccept(_)
                                        | GameControl::Resign(_)
                                        | GameControl::DrawReject(_)
                                        | GameControl::TakebackReject(_)
                                        | GameControl::Abort(_) => {
                                            game_state.game_control_pending().set(None);
                                        }
                                    }
                                }
                            });
                        }
                        GameReaction::Started => {
                            batch(|| {
                                game_state.reset_from_response(&gar.game);
                                timer.update_from(&gar.game);
                            });
                        }
                        GameReaction::TimedOut => {
                            batch(|| {
                                game_state.reset_from_response(&gar.game);
                                timer.update_from(&gar.game);
                            });
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
    let game_state = expect_context::<GameStateStore>();
    let controls_signal = expect_context::<ControlsSignal>();
    let vertical = true;
    let top_color = Signal::derive(move || player_color().opposite_color());
    let is_finished = game_state.is_finished();
    let show_controls = Signal::derive(move || !controls_signal.hidden.get() || is_finished.get());
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
        <div class="flex flex-col flex-grow h-full min-h-0">
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
                    <div class="grid grid-cols-5 gap-1 px-1 pb-1 [&>*]:w-full">
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
