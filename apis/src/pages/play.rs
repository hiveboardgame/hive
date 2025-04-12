use crate::{
    common::{GameReaction, MoveConfirm},
    components::{
        atoms::history_button::{HistoryButton, HistoryNavigation},
        layouts::base_layout::{ControlsSignal, OrientationSignal},
        molecules::{
            analysis_and_download::AnalysisAndDownload, control_buttons::ControlButtons,
            game_info::GameInfo, user_with_rating::UserWithRating,
        },
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve},
            side_board::SideboardTabs,
            unstarted::Unstarted,
        },
    },
    functions::games::get::get_game_from_nanoid,
    providers::{
        config::Config, game_state::GameStateSignal, timer::TimerSignal,
        websocket::WebsocketContext, ApiRequestsProvider, AuthContext, SoundType, Sounds,
        UpdateNotifier,
    },
    websocket::client_handlers::game::{reset_game_state, reset_game_state_for_takeback},
};
use hive_lib::{Color, GameControl, GameResult, GameStatus, Turn};
use leptos::{either::EitherOf3, prelude::*};
use leptos_router::hooks::{use_navigate, use_params_map};
use shared_types::{GameId, GameStart, TournamentGameResult};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct CurrentConfirm(pub Memo<MoveConfirm>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();

    let params = use_params_map();
    let game_id = Memo::new(move |_| {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    });
    let orientation_signal = expect_context::<OrientationSignal>();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let current_confirm = Memo::new(move |_| {
        let preferred_confirms = config().confirm_mode;
        game_state
            .signal
            .get()
            .get_game_speed()
            .and_then(|game_speed| preferred_confirms.get(&game_speed).cloned())
            .unwrap_or_default()
    });
    provide_context(CurrentConfirm(current_confirm));
    let user = auth_context.user;
    let white_and_black = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));
    let user_is_player = Signal::derive(move || {
        user().and_then(|user| {
            let (white_id, black_id) = white_and_black();
            if Some(user.id) == black_id || Some(user.id) == white_id {
                Some((user.username, user.id))
            } else {
                None
            }
        })
    });
    let game_updater = expect_context::<UpdateNotifier>();
    provide_context(TimerSignal::new());
    let timer = expect_context::<TimerSignal>();
    let api = expect_context::<ApiRequestsProvider>();
    let ws = expect_context::<WebsocketContext>();
    let ws_ready = ws.ready_state;
    Effect::watch(
        move || {
            ws_ready();
            game_id()
        },
        move |game_id, _, _| {
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
                                game_state.set_pending_gc(gc.clone())
                            }
                            _ => {}
                        }
                    }
                };
            });
        },
        true,
    );
    //HB handler
    Effect::watch(
        move || game_updater.heartbeat.get(),
        move |hb, _, _| {
            timer.update_from_hb(hb.clone());
        },
        false,
    );
    let player_color = Memo::new(move |_| {
        user().map_or(Color::White, |user| {
            let black_id = white_and_black().1;
            match Some(user.id) == black_id {
                true => Color::Black,
                false => Color::White,
            }
        })
    });
    let parent_container_style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex flex-col"
        } else {
            "grid grid-cols-board xl:grid-cols-board-xl grid-rows-6 pr-2"
        }
    };
    let go_to_game = Callback::new(move |()| {
        if game_state.signal.get_untracked().is_last_turn() {
            game_state.view_game();
        }
    });
    let top_color = move || player_color().opposite_color();
    let controls_signal = expect_context::<ControlsSignal>();
    let show_controls =
        Signal::derive(move || !controls_signal.hidden.get() || game_state.is_finished()());
    let show_board = create_read_slice(game_state.signal, |gs| {
        !gs.game_response.as_ref().is_some_and(|gr| {
            gr.game_start == GameStart::Ready
                && matches!(gr.game_status, GameStatus::NotStarted)
                && gr.tournament_game_result == TournamentGameResult::Unknown
        })
    });
    let sounds = expect_context::<Sounds>();
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
                            if game_state.signal.get_untracked().state.history.moves
                                != gar.game.history
                            {
                                match turn {
                                    Turn::Move(piece, position) => {
                                        game_state.play_turn(piece, position)
                                    }
                                    _ => unreachable!(),
                                };
                            }
                        }
                        GameReaction::Control(game_control) => {
                            game_state.set_pending_gc(game_control.clone());

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
                                | GameControl::TakebackReject(_) => {}
                                GameControl::Abort(_) => {
                                    let navigate = use_navigate();
                                    navigate("/", Default::default());
                                }
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
    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 select-none bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=move || !orientation_signal.orientation_vertical.get()
                fallback=move || {
                    view! {
                        <div class="flex flex-col flex-grow h-full min-h-0">
                            <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                                <Show when=show_controls>
                                    <div class="flex flex-row-reverse justify-between items-center shrink">
                                        <AnalysisAndDownload />
                                        <Show when=move || user_is_player().is_some()>
                                            <ControlButtons />
                                        </Show>
                                    </div>
                                </Show>

                                <div class="flex justify-between ml-1 h-full max-h-16">
                                    <Reserve alignment=Alignment::SingleRow color=top_color() />
                                    <DisplayTimer vertical=true placement=Placement::Top />
                                </div>
                                <div class="flex gap-1 border-b-[1px] border-dashed border-gray-500 justify-between px-1 bg-inherit">
                                    <UserWithRating
                                        side=top_color()
                                        is_tall=orientation_signal.orientation_vertical
                                    />
                                    <GameInfo />
                                </div>

                            </div>
                            <BoardOrUnstarted
                                show_board=show_board()
                                is_vertical=orientation_signal.orientation_vertical.get()
                            />
                            <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                                <div class="flex gap-1 border-t-[1px] border-dashed border-gray-500">
                                    <UserWithRating
                                        side=player_color()
                                        is_tall=orientation_signal.orientation_vertical
                                    />
                                </div>
                                <div class="flex justify-between mb-2 ml-1 h-full max-h-16">
                                    <Reserve alignment=Alignment::SingleRow color=player_color() />
                                    <DisplayTimer vertical=true placement=Placement::Bottom />
                                </div>
                                <Show when=show_controls>
                                    <div class="grid grid-cols-4 gap-8 pb-1">
                                        <HistoryButton action=HistoryNavigation::First />
                                        <HistoryButton action=HistoryNavigation::Previous />
                                        <HistoryButton
                                            action=HistoryNavigation::Next
                                            post_action=go_to_game
                                        />
                                        <HistoryButton action=HistoryNavigation::MobileLast />
                                    </div>
                                </Show>

                            </div>
                        </div>
                    }
                }
            >
                <BoardOrUnstarted
                    show_board=show_board()
                    is_vertical=orientation_signal.orientation_vertical.get()
                />
                <div class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full">
                    <DisplayTimer placement=Placement::Top vertical=false />
                    <SideboardTabs player_color />
                    <DisplayTimer placement=Placement::Bottom vertical=false />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn BoardOrUnstarted(show_board: bool, is_vertical: bool) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_updater = expect_context::<UpdateNotifier>();
    let user = expect_context::<AuthContext>().user;
    let white = create_read_slice(game_state.signal, |gs| {
        (
            gs.white_id,
            gs.game_response.clone().map(|gr| gr.white_player.username),
        )
    });
    let black = create_read_slice(game_state.signal, |gs| {
        (
            gs.black_id,
            gs.game_response.clone().map(|gr| gr.black_player.username),
        )
    });
    let params = use_params_map();
    let game_id = Memo::new(move |_| {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    });
    let user_is_player = Signal::derive(move || {
        user().and_then(|user| {
            let (white_id, black_id) = (white().0, black().0);
            if Some(user.id) == black_id || Some(user.id) == white_id {
                Some((user.username, user.id))
            } else {
                None
            }
        })
    });
    if show_board {
        if is_vertical {
            EitherOf3::A(view! {
                <GameInfo extend_tw_classes="absolute pl-4 pt-2 bg-board-dawn dark:bg-board-twilight" />
                <Board />
            })
        } else {
            EitherOf3::B(view! { <Board overwrite_tw_classes="flex grow min-h-0" /> })
        }
    } else {
        EitherOf3::C(view! {
            <Unstarted
                user_is_player=user_is_player().is_some()
                white=white()
                black=black()
                game_id=game_id()
                ready=game_updater.tournament_ready.into()
            />
        })
    }
}
