use crate::{
    common::MoveConfirm,
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
    providers::{config::Config, game_state::GameStateSignal, AuthContext},
};
use hive_lib::{Color, GameStatus, Position};
use leptos::prelude::*;
use shared_types::{GameStart, TournamentGameResult};

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[derive(Clone)]
pub struct CurrentConfirm(pub Memo<MoveConfirm>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let orientation_signal = expect_context::<OrientationSignal>();
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let current_confirm = Memo::new(move |_| {
        let preferred_confirms = config().unwrap_or_default().confirm_mode;
        game_state
            .signal
            .get()
            .get_game_speed()
            .and_then(|game_speed| preferred_confirms.get(&game_speed).cloned())
            .unwrap_or_default()
    });
    provide_context(CurrentConfirm(current_confirm));
    let user = move || match auth_context.user.get() {
        Some(Ok(user)) => Some(user),
        _ => None,
    };
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
        let mut game_state = expect_context::<GameStateSignal>();
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

    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 select-none bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=orientation_signal.orientation_vertical
                fallback=move || {
                    view! {
                        <Show
                            when=show_board
                            fallback=move || {
                                view! { <Unstarted user_is_player /> }
                            }
                        >

                            <GameInfo extend_tw_classes="absolute pl-4 pt-2 bg-board-dawn dark:bg-board-twilight" />
                            <Board />
                        </Show>
                        <div class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full">
                            <DisplayTimer placement=Placement::Top vertical=false />
                            <SideboardTabs player_color />
                            <DisplayTimer placement=Placement::Bottom vertical=false />
                        </div>
                    }
                }
            >

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
                            <Reserve
                                alignment=Alignment::SingleRow
                                color=top_color()
                                analysis=false
                            />
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

                    <Show
                        when=show_board
                        fallback=move || {
                            view! {
                                <Unstarted
                                    overwrite_tw_classes="flex grow min-h-0 h-[100dvh] justify-center items-center"
                                    user_is_player
                                />
                            }
                        }
                    >

                        <Board overwrite_tw_classes="flex grow min-h-0" />
                    </Show>
                    <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                        <div class="flex gap-1 border-t-[1px] border-dashed border-gray-500">
                            <UserWithRating
                                side=player_color()
                                is_tall=orientation_signal.orientation_vertical
                            />
                        </div>
                        <div class="flex justify-between mb-2 ml-1 h-full max-h-16">
                            <Reserve
                                alignment=Alignment::SingleRow
                                color=player_color()
                                analysis=false
                            />
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
            </Show>
        </div>
    }
}
