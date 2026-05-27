use crate::{
    common::{GameReaction, MoveConfirm, PieceType},
    components::{
        atoms::history_button::{sync_play_move_query, HistoryButton, HistoryNavigation},
        layouts::base_layout::{ControlsSignal, OrientationSignal},
        molecules::{
            analysis_and_download::AnalysisAndDownload,
            control_buttons::ControlButtons,
            game_info::GameInfo,
            history_controls::scroll_active_history_move_into_view,
            user_with_rating::UserWithRating,
        },
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve},
            side_board::{move_query_signal, SideboardTabs, TabView},
            unstarted::Unstarted,
        },
    },
    functions::games::get::get_game_from_nanoid,
    providers::{
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
use hive_lib::{Color, GameControl, GameResult, GameStatus, Turn};
use leptos::{ev::keydown, prelude::*};
use leptos_router::hooks::{use_params_map, use_query_map};
use leptos_use::{use_event_listener, use_window};
use shared_types::{GameId, GameStart};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct CurrentConfirm(pub Memo<MoveConfirm>);

#[component]
pub fn Play() -> impl IntoView {
    provide_context(TimerSignal::new());
    let timer = expect_context::<TimerSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
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
        if orientation_signal.orientation_vertical.get() {
            "flex flex-col"
        } else {
            "grid grid-cols-board xl:grid-cols-board-xl grid-rows-6 pr-2"
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

    // Global keyboard handler: ArrowLeft enters history mode (from anywhere) and steps back;
    // ArrowRight steps forward when already in history.
    // use_window() returns None on SSR so use_event_listener becomes a no-op there.
    let body = use_window().document().body();
    _ = use_event_listener(body, keydown, move |evt| {
        let key = evt.key();
        if key != "ArrowLeft" && key != "ArrowRight" {
            return;
        }
        if evt.alt_key() || evt.ctrl_key() || evt.meta_key() || evt.shift_key() {
            return;
        }
        // Don't steal arrow keys from text inputs (chat, search fields, etc.).
        if let Some(target) = evt.target() {
            let tag = target
                .unchecked_ref::<web_sys::Element>()
                .tag_name()
                .to_uppercase();
            if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                return;
            }
        }
        evt.prevent_default();
        if key == "ArrowLeft" {
            let has_turns = game_state.signal.with_untracked(|gs| gs.state.turn > 0);
            if !has_turns {
                return;
            }
            // When entering from live game view, position to the last turn first so
            // previous_history_turn() steps back one from the most-recent move.
            // Don't re-trigger this if already in History view — that would teleport
            // the user back to the end when they've navigated to the beginning.
            let entering = game_state
                .signal
                .with_untracked(|gs| matches!(gs.view, View::Game));
            if entering {
                game_state.signal.update(|gs| {
                    gs.view_history();
                    gs.history_turn = gs.state.turn.checked_sub(1);
                });
            }
            let can_step_back = game_state
                .signal
                .with_untracked(|gs| matches!(gs.history_turn, Some(turn) if turn > 0));
            if can_step_back {
                game_state.previous_history_turn();
            } else if !entering {
                return;
            }
            tab.set(TabView::History);
            controls_signal.hidden.set(false);
            sync_play_move_query(game_state, &set_move);
            scroll_active_history_move_into_view();
        } else {
            // ArrowRight: navigate forward only when already browsing history.
            let in_history = game_state
                .signal
                .with_untracked(|gs| matches!(gs.view, View::History));
            if in_history {
                game_state.next_history_turn();
                if game_state.signal.with_untracked(|gs| gs.is_last_turn()) {
                    game_state.view_game();
                }
                sync_play_move_query(game_state, &set_move);
                scroll_active_history_move_into_view();
            }
        }
    });

    view! {
        <div class=move || {
            format!(
                "h-[100dvh] pt-10 standalone:h-[var(--app-height)] select-none bg-board-dawn dark:bg-board-twilight {}",
                parent_container_style(),
            )
        }>
            <Show
                when=orientation_signal.orientation_vertical
                fallback=move || {
                    view! {
                        <HorizontalLayout
                            show_board
                            player_color
                            user_is_player
                            game_id
                            white_and_black_ids
                            tab
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
                />

            </Show>
        </div>
    }
}

#[component]
fn BoardOrUnstarted(
    show_board: Signal<bool>,
    user_is_player: Signal<bool>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    game_id: Memo<GameId>,
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
            <Board />
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
    tab: RwSignal<TabView>,
) -> impl IntoView {
    let vertical = false;
    let config = expect_context::<Config>().0;
    let background_style = Signal::derive(move || {
        let bg = config.with(|c| c.tile.get_effective_background_color(c.prefers_dark));
        format!("background-color: {bg}")
    });
    view! {
        <GameInfo extend_tw_classes="absolute pl-4 pt-2 bg-transparent" />
        <BoardOrUnstarted show_board user_is_player game_id white_and_black_ids />
        <div
            class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full"
            style=background_style
        >
            <DisplayTimer placement=Placement::Top vertical />
            <SideboardTabs player_color tab />
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
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let controls_signal = expect_context::<ControlsSignal>();
    let vertical = true;
    let go_to_game = Callback::new(move |()| {
        if game_state.signal.with_untracked(|gs| gs.is_last_turn()) {
            game_state.view_game();
        }
    });
    let top_color = Signal::derive(move || player_color().opposite_color());
    let show_controls =
        Signal::derive(move || !controls_signal.hidden.get() || game_state.is_finished()());
    view! {
        <div class="flex flex-col flex-grow h-full min-h-0">
            <div class="flex flex-col shrink bg-board-dawn dark:bg-reserve-twilight">
                <Show when=show_controls>
                    <div class="flex flex-row-reverse justify-between items-center shrink">
                        <AnalysisAndDownload />
                        <Show when=user_is_player>
                            <ControlButtons />
                        </Show>
                    </div>
                </Show>

                <div class="flex justify-between ml-1 h-full max-h-16">
                    <Reserve alignment=Alignment::SingleRow color=top_color />
                    <DisplayTimer vertical=true placement=Placement::Top />
                </div>
                <div class="flex gap-1 justify-between px-1 border-gray-500 border-dashed border-b-[1px] bg-inherit">
                    <UserWithRating side=top_color vertical />
                    <GameInfo />
                </div>

            </div>
            <BoardOrUnstarted show_board user_is_player game_id white_and_black_ids />
            <div class="flex flex-col shrink bg-board-dawn dark:bg-reserve-twilight">
                <div class="flex gap-1 border-gray-500 border-dashed border-t-[1px]">
                    <UserWithRating side=player_color vertical />
                </div>
                <div class="flex justify-between mb-2 ml-1 h-full max-h-16">
                    <Reserve alignment=Alignment::SingleRow color=player_color />
                    <DisplayTimer vertical=true placement=Placement::Bottom />
                </div>
                <Show when=show_controls>
                    <div class="grid grid-cols-4 gap-8 pb-1">
                        <HistoryButton action=HistoryNavigation::First />
                        <HistoryButton action=HistoryNavigation::Previous />
                        <HistoryButton action=HistoryNavigation::Next post_action=go_to_game />
                        <HistoryButton action=HistoryNavigation::MobileLast />
                    </div>
                </Show>

            </div>
        </div>
    }
}
